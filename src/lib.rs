#![cfg_attr(debug_assertions, allow(dead_code, unused))]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::slice::Iter;

#[macro_use] extern crate failure;
use failure::Error;

use indexmap::{self, IndexMap, IndexSet};
use memchr::Memchr;

pub mod args;
use crate::args::OpName;

type TextVec = Vec<u8>;
type TextSlice = [u8];
type LineIterator<'a> = Box<dyn Iterator<Item = &'a TextSlice> + 'a>;

type UnionSet = IndexSet<TextVec>;

#[derive(PartialEq)]
enum FoundIn {
    One,
    Many,
}
type CountedSet = IndexMap<TextVec, FoundIn>;

#[derive(Default)]
struct SingleSet(CountedSet);

#[derive(Default)]
struct MultipleSet(CountedSet);

type SliceSet<'data> = IndexSet<&'data TextSlice>;

#[derive(Default)]
struct DiffSet<'data>(SliceSet<'data>);

#[derive(Default)]
struct IntersectSet<'data>(SliceSet<'data>);

pub type SetOpResult = Result<(), Error>;
/// Calculates and prints the set operation named by `op`. Each file in `files`
/// is treated as a set of lines:
///
/// * `union` prints the lines that occur in any file,
/// * `intersect` prints the lines that occur in all files,
/// * `diff` prints the lines that occur in the first file and no other,
/// * `single` prints the lines that occur in exactly one file, and
/// * `multiple` prints the lines that occur in more than one file.
pub fn do_calculation(operation: OpName, files: Vec<PathBuf>) -> SetOpResult {
    use std::mem::drop;
    let mut operands = contents_iter(files);
    let first = match operands.next() {
        None => return Ok(()),
        Some(operand) => operand?,
    };
    match operation {
        OpName::Intersect => calculate_and_print(&mut IntersectSet::init(&first), operands)?,
        OpName::Diff => calculate_and_print(&mut DiffSet::init(&first), operands)?,
        OpName::Union => {
            let mut set = UnionSet::init(&first);
            drop(first);
            calculate_and_print(&mut set, operands)?;
        }
        OpName::Single => {
            let mut set = SingleSet::init(&first);
            drop(first);
            calculate_and_print(&mut set, operands)?;
        }
        OpName::Multiple => {
            let mut set = MultipleSet::init(&first);
            drop(first);
            calculate_and_print(&mut set, operands)?;
        }
    }
    Ok(())
}

fn contents_iter(files: Vec<PathBuf>) -> ContentsIter {
    ContentsIter{files: files.into_iter() }
}
struct ContentsIter {
    files: std::vec::IntoIter<PathBuf>,
}
impl Iterator for ContentsIter {
    type Item = Result<TextVec, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let path = self.files.next()?;
        Some(match fs::read(&path) {
            Ok(contents) => Ok(contents),
            Err(io_err) => {
                let path = path.to_string_lossy();
                Err(format_err!("Can't read file `{}`: {}", path, io_err))
            }
        })
    }
}

fn calculate(set: &mut impl SetExpression, operands: ContentsIter) -> SetOpResult {
    for operand in operands {
        set.operate(&operand?);
    }
    set.finish();
    Ok(())
}

fn calculate_and_print(set: &mut impl SetExpression, operands: ContentsIter) -> SetOpResult {
    calculate(set, operands)?;
    let stdout_for_locking = io::stdout();
    let mut stdout = stdout_for_locking.lock();
    for line in set.iter() {
        stdout.write_all(line)?;
    }
    Ok(())
}

trait SetExpression {
    fn operate(&mut self, other: &TextSlice);
    fn finish(&mut self) {}
    fn iter(&self) -> LineIterator;
}

// Sets are implemented as variations on the `IndexMap` type, a hash that remembers
// the order in which keys were inserted, since our 'sets' are equipped with an
// ordering on the members.
//
trait LineSet<'data>: Default {
    // The only method that implementations need to define is `insert_line`
    fn insert_line(&mut self, line: &'data TextSlice);

    // The `insert_all_lines` method breaks `text` down into lines and inserts
    // each of them into `self`
    fn insert_all_lines(&mut self, text: &'data TextSlice) {
        let mut begin = 0;
        for end in Memchr::new(b'\n', text) {
            self.insert_line(&text[begin..=end]);
            begin = end + 1;
        }
        //FIXME: this leaves the last line of the file without a newline. Given that
        // fs::read allocates an extra byte at the end of the returned vector, we could
        // just add a newline there.  But that's pretty fragile!
        if begin < text.len() {
            self.insert_line(&text[begin..]);
        }
    }
    // We initialize a `LineSet` from `text` by inserting every line contained
    // in text into an empty hash.
    fn init(text: &'data TextSlice) -> Self {
        let mut set = Self::default();
        set.insert_all_lines(text);
        set
    }
}

// The simplest `LineSet` is a `SliceSet`, whose members (hash keys) are slices
// borrowed from a text string, each slice corresponding to a line.
//
impl<'data> LineSet<'data> for SliceSet<'data> {
    fn insert_line(&mut self, line: &'data TextSlice) {
        self.insert(line);
    }
}

// The next simplest set is a `UnionSet`, which we use to calculate the union
// of the lines which occur in at least one of a sequence of files. Rather than
// keep the text of all files in memory, we allocate a `TextVec` for each set member.
//
impl<'data> LineSet<'data> for UnionSet {
    fn insert_line(&mut self, line: &'data TextSlice) {
        self.insert(line.to_vec());
    }
}
impl SetExpression for UnionSet {
    fn operate(&mut self, other: &TextSlice) {
        self.insert_all_lines(&other);
    }
    fn iter(&self) -> LineIterator {
        Box::new(self.iter().map(|v| v.as_slice()))
    }
}

/// We use a `SingleSet` to keep track of the lines which occur in exactly one of
/// the given files, and a `MultipleSet` to keep track of those that occur in more
/// than one file.  Underlying a `SingleSet` or a `MultipleSet` is an `IndexMap`
/// whose values are either `FoundIn::One` for lines that occur in only one file
/// or `FoundIn:Many` for lines that occur in multiple files. (If a line occurs
/// more than once in just one particular file, we still count it as occuring in
/// a single file.)
///
/// For the first operand we set every line's value to `FoundIn::One`, and if it
/// is found in a subsequent file we set its value to `FoundIn::Many`.  The only
/// implementation difference between a `SingleSet` and a `MultipleSet` is that
/// at the end of the calculation we retain for a `SingleSet` the keys with a
/// `FoundIn::One` value and for a `MultipleSet` the keys with a `FoundIn::Many`
/// value. We use a macro to avoid two chunks of code differing in a single line.

macro_rules! impl_counted_set {
    ($CountedSet:ident, $count:expr) => {
        impl<'data> LineSet<'data> for $CountedSet {
            fn insert_line(&mut self, line: &'data TextSlice) {
                self.0.insert(line.to_vec(), FoundIn::One);
            }
        }
        impl SetExpression for $CountedSet {
            /// If a line occurs in `other` but not `self`,
            /// we insert it with a `true` value; if it
            /// occurs in both, we set its value to `false`
            fn operate(&mut self, other: &TextSlice) {
                let other = SliceSet::init(other);
                for line in other.iter() {
                    if self.0.contains_key(*line) {
                        self.0.insert(line.to_vec(), FoundIn::Many);
                    } else {
                        self.0.insert(line.to_vec(), FoundIn::One);
                    }
                }
            }
            /// Remove the unwanted values
            fn finish(&mut self) {
                self.0.retain(|_k, v| *v == $count);
            }
            fn iter(&self) -> LineIterator {
                Box::new(self.0.keys().map(|k| k.as_slice()))
            }
        }
    };
}
impl_counted_set!(SingleSet, FoundIn::One);
impl_counted_set!(MultipleSet, FoundIn::Many);

/// For an `IntersectSet` or a `DiffSet`, all result lines will be from the
/// first file operand, so we can avoid additional allocations by keeping its
/// text in memory and using subslices of its text as the members of the set.
///
/// For subsequent operands, we take a `SliceSet` `s` of the operand's text and
/// (for an `IntersectSet`) keep only those lines that occur in `s` or (for a
/// `DiffSet`) remove the lines that occur in `s`. Again we use a macro to avoid
/// two chunks of code differing in a single line.
macro_rules! impl_waning_set {
    ($WaningSet:ident, $filter:ident) => {
        impl<'data> LineSet<'data> for $WaningSet<'data> {
            fn insert_line(&mut self, line: &'data TextSlice) {
                self.0.insert(line);
            }
        }
        impl<'data> SetExpression for $WaningSet<'data> {
            /// Remove (for DiffSet) or retain (for IntersectSet) the elements
            /// of `other`
            fn operate(&mut self, other: &TextSlice) {
                let other = SliceSet::init(other);
                $filter(&mut self.0, &other);
            }
            fn iter<'me>(&'me self) -> LineIterator<'me> {
                Box::new(self.0.iter().cloned())
            }
        }
    };
}

impl_waning_set!(IntersectSet, intersect);

fn intersect(set: &mut SliceSet, other: &SliceSet) {
    set.retain(|x| other.contains(x));
}

impl_waning_set!(DiffSet, difference);

fn difference(set: &mut SliceSet, other: &SliceSet) {
    set.retain(|x| !other.contains(x));
}

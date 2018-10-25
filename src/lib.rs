#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![deny(unused_must_use)]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]

use std::io::Write;

#[macro_use]
extern crate failure;
use failure::Error;

use indexmap::{IndexMap, IndexSet};
use memchr::Memchr;

pub mod args;
use crate::args::OpName;
pub mod sio;

type LineIterator<'a> = Box<dyn Iterator<Item = &'a [u8]> + 'a>;

type UnionSet = IndexSet<Vec<u8>>;

#[derive(PartialEq)]
enum FoundIn {
    One,
    Many,
}
type CountedSet = IndexMap<Vec<u8>, FoundIn>;

#[derive(Default)]
struct SingleSet(CountedSet);

#[derive(Default)]
struct MultipleSet(CountedSet);

type SliceSet<'data> = IndexSet<&'data [u8]>;

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
pub fn do_calculation(
    operation: OpName,
    operands: impl IntoIterator<Item = Result<Vec<u8>, Error>>,
    output: &mut impl Write,
) -> SetOpResult {
    let mut operands = operands.into_iter();
    let first = match operands.next() {
        None => return Ok(()),
        Some(operand) => operand?,
    };
    let mut set: Box<dyn SetExpression> = match operation {
        OpName::Intersect => Box::new(IntersectSet::borrowing(&first)),
        OpName::Diff => Box::new(DiffSet::borrowing(&first)),
        OpName::Union => Box::new(UnionSet::consuming(first)),
        OpName::Single => Box::new(SingleSet::consuming(first)),
        OpName::Multiple => Box::new(MultipleSet::consuming(first)),
    };

    for operand in operands {
        set.operate(operand?.as_ref());
    }
    set.finish();

    for line in set.iter() {
        output.write_all(line)?;
    }
    output.flush()?;

    Ok(())
}

trait SetExpression {
    fn operate(&mut self, other: &[u8]);
    fn finish(&mut self) {}
    fn iter(&self) -> LineIterator;
}

// Sets are implemented as variations on the `IndexMap` type, a hash that remembers
// the order in which keys were inserted, since our 'sets' are equipped with an
// ordering on the members.
//
trait LineSet<'data>: Default {
    // The only method that implementations need to define is `insert_line`
    fn insert_line(&mut self, line: &'data [u8]);

    // The `insert_all_lines` method breaks `text` down into lines and inserts
    // each of them into `self`
    fn insert_all_lines(&mut self, text: &'data [u8]) {
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
    fn borrowing(text: &'data [u8]) -> Self {
        let mut set = Self::default();
        set.insert_all_lines(text);
        set
    }
}

/// A waxing set's members are allocated vectors, so its lifetime is independant
/// of its first operand. To conserve space, we drop that operand after reading it.
trait ConsumingSet: for<'a> LineSet<'a> + Default {
    fn consuming(text: impl Into<Vec<u8>>) -> Self {
        let mut set = Self::default();
        set.insert_all_lines(&text.into());
        set
    }
}

// The simplest `LineSet` is a `SliceSet`, whose members (hash keys) are slices
// borrowed from a text string, each slice corresponding to a line.
//
impl<'data> LineSet<'data> for SliceSet<'data> {
    fn insert_line(&mut self, line: &'data [u8]) {
        self.insert(line);
    }
}

// The next simplest set is a `UnionSet`, which we use to calculate the union
// of the lines which occur in at least one of a sequence of files. Rather than
// keep the text of all files in memory, we allocate a `Vec<u8>` for each set member.
//
impl ConsumingSet for UnionSet {}
impl<'data> LineSet<'data> for UnionSet {
    fn insert_line(&mut self, line: &'data [u8]) {
        self.insert(line.to_vec());
    }
}
impl SetExpression for UnionSet {
    fn operate(&mut self, other: &[u8]) {
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
        impl ConsumingSet for $CountedSet {}
        impl<'data> LineSet<'data> for $CountedSet {
            fn insert_line(&mut self, line: &'data [u8]) {
                self.0.insert(line.to_vec(), FoundIn::One);
            }
        }
        impl SetExpression for $CountedSet {
            /// If a line occurs in `other` but not `self`,
            /// we insert it with a `true` value; if it
            /// occurs in both, we set its value to `false`
            fn operate(&mut self, other: &[u8]) {
                let other = SliceSet::borrowing(other);
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
            fn insert_line(&mut self, line: &'data [u8]) {
                self.0.insert(line);
            }
        }
        impl<'data> SetExpression for $WaningSet<'data> {
            /// Remove (for DiffSet) or retain (for IntersectSet) the elements
            /// of `other`
            fn operate(&mut self, other: &[u8]) {
                let other = SliceSet::borrowing(other);
                $filter(&mut self.0, &other);
            }
            fn iter(&self) -> LineIterator {
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

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;
    use std::convert::AsRef;

    fn calc(operation: OpName, operands: &Vec<&[u8]>) -> Vec<u8> {
        let mut answer = Vec::<u8>::new();
        let operands = operands.iter().map(|s| Ok(s.to_vec()));
        do_calculation(operation, operands, &mut answer).unwrap();
        answer
    }

    use self::OpName::*;

    #[test]
    fn given_a_single_argument_all_ops_but_multiple_return_its_lines_in_order_without_dups() {
        let arg: Vec<&[u8]> = vec![b"xxx\nabc\nxxx\nyyy\nxxx\nabc\n"];
        let uniq = b"xxx\nabc\nyyy\n";
        for op in [Intersect, Union, Diff, Single, Multiple].iter() {
            match op {
                Intersect | Union | Diff | Single => assert_eq!(calc(*op, &arg), uniq),
                Multiple => assert_eq!(calc(*op, &arg), b""),
            }
        }
    }
    #[test]
    fn results_for_each_operation() {
        let args: Vec<&[u8]> = vec![
            b"xyz\nabc\nxy\nxz\nx\n", // Strings containing "x" (and "abc")
            b"xyz\nabc\nxy\nyz\ny\n", // Strings containing "y" (and "abc")
            b"xyz\nabc\nxz\nyz\nz\n", // Strings containing "z" (and "abc")
        ];
        assert_eq!(calc(Union, &args), b"xyz\nabc\nxy\nxz\nx\nyz\ny\nz\n");
        assert_eq!(calc(Intersect, &args), b"xyz\nabc\n");
        assert_eq!(calc(Diff, &args), b"x\n");
        assert_eq!(calc(Single, &args), b"x\ny\nz\n");
        assert_eq!(calc(Multiple, &args), b"xyz\nabc\nxy\nxz\nyz\n");
    }
}

//! The `do_calculation` function is the kernel of the appliction.  The `args` module parses
//! the command line, and the `io` module hides I/O details.
//!
//! Current Limitations:
//! * Currently a "line" is zero or more non-newline bytes followed by a newline.
//!   That's a problem for little-endian UTF-16.  Eventually we want to use BOM
//!   sniffing to detect UTF-16LE, UTF16BE, and UTF8 so we can
//!   * allow files of different formats on the command line, and
//!   * make our output compatible with the format of the first operand.

#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![deny(unused_must_use)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![warn(missing_docs)]

use std::io::Write;
use std::vec::Vec;

#[macro_use]
extern crate failure;

use indexmap::{IndexMap, IndexSet};
use memchr::Memchr;

pub mod args;
use crate::args::OpName;
pub mod io;

/// The `LineIterator` type is used to return the value of a `SetExpression` `s`:
/// `s.iter()` returns an iterator over the lines (elements) of `s`.
pub type LineIterator<'a> = Box<dyn Iterator<Item = &'a [u8]> + 'a>;

type UnionSet = IndexSet<Vec<u8>>;

#[derive(PartialEq)]
enum FoundIn {
    One,
    Many,
}
type CountedSet = IndexMap<Vec<u8>, FoundIn>;

type SliceSet<'data> = IndexSet<&'data [u8]>;

#[derive(Default)]
struct DiffSet<'data>(SliceSet<'data>);

#[derive(Default)]
struct IntersectSet<'data>(SliceSet<'data>);

/// Calculates and prints the set operation named by `op`. Each file in `files`
/// is treated as a set of lines:
///
/// * `OpName::Union` prints the lines that occur in any file,
/// * `OpName::Intersect` prints the lines that occur in all files,
/// * `OpName::Diff` prints the lines that occur in the first file and no other,
/// * `OpName::Single` prints the lines that occur in exactly one file, and
/// * `OpName::Multiple` prints the lines that occur in more than one file.
///
/// **Every** line in each element of `operands` must end in `b'\n'`, including
/// the element's last line.
pub fn do_calculation(
    operation: OpName,
    operands: impl IntoIterator<Item = Result<Vec<u8>, failure::Error>>,
    output: impl FnOnce(LineIterator) -> Result<(), failure::Error>,
) -> Result<(), failure::Error> {
    let mut operands = operands.into_iter();
    let first = match operands.next() {
        None => return Ok(()),
        Some(operand) => operand?,
    };
    let mut set: Box<dyn SetExpression> = match operation {
        OpName::Intersect => Box::new(IntersectSet::with(&first)),
        OpName::Diff => Box::new(DiffSet::with(&first)),
        OpName::Union => Box::new(UnionSet::with(&first)),
        OpName::Single | OpName::Multiple => Box::new(CountedSet::with(&first)),
    };

    for operand in operands {
        set.operate(operand?.as_ref());
    }
    match operation {
        OpName::Single => set.keep_members(FoundIn::One),
        OpName::Multiple => set.keep_members(FoundIn::Many),
        _ => {}
    };

    output(set.iter())
}

trait SetExpression {
    fn operate(&mut self, other: &[u8]);
    fn iter(&self) -> LineIterator;

    // This is a code smell, since only CountedSet needs it. But I don't know a
    // better way
    fn keep_members(&mut self, _keep: FoundIn) {}
}

// Sets are implemented as variations on the `IndexMap` type, a hash that remembers
// the order in which keys were inserted, since our 'sets' are equipped with an
// ordering on the members.
//
trait LineSet<'data>: Default {
    // The only method that implementations need to define is `insert_line`
    fn insert_line(&mut self, line: &'data [u8]);

    // The `insert_all_lines` method breaks `text` down into lines and inserts
    // each of them into `self`, including the ending `b'\n'`.
    fn insert_all_lines(&mut self, text: &'data [u8]) {
        let mut begin = 0;
        for end in Memchr::new(b'\n', text) {
            self.insert_line(&text[begin..=end]); // keep the newline
            begin = end + 1;
        }
    }
    // The initial value is the set of all lines in the first operand
    fn with(text: &'data [u8]) -> Self {
        let mut set = Self::default();
        set.insert_all_lines(text);
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
        Box::new(self.iter().map(Vec::as_slice))
    }
}

/// We use a `CountedSet` to keep track, for each line seen, whether the line
/// occurs in exactly one of the given files, or of more than one of them.
/// A `CountedSet` is an `IndexMap` whose values are either `FoundIn::One` for
/// lines that occur in only one file or `FoundIn:Many` for lines that occur
/// in multiple files. (If a line occurs more than once in just one particular
/// file, we still count it as occuring in a single file.)
///
/// For the first operand we set every line's value to `FoundIn::One`, and if it
/// is found in a subsequent file we set its value to `FoundIn::Many`.
impl<'data> LineSet<'data> for CountedSet {
    fn insert_line(&mut self, line: &'data [u8]) {
        self.insert(line.to_vec(), FoundIn::One);
    }
}
impl SetExpression for CountedSet {
    /// If a line occurs in `other` but not `self`,
    /// we insert it with a `FoundIn::One` value; if it
    /// occurs in both, we set its value to `FoundIn::Many`
    fn operate(&mut self, other: &[u8]) {
        let other = SliceSet::with(other);
        for line in other.iter() {
            if self.contains_key(*line) {
                self.insert(line.to_vec(), FoundIn::Many);
            } else {
                self.insert(line.to_vec(), FoundIn::One);
            }
        }
    }
    /// Remove the unwanted values
    fn keep_members(&mut self, keep: FoundIn) {
        self.retain(|_k, v| *v == keep);
    }
    fn iter(&self) -> LineIterator {
        Box::new(self.keys().map(Vec::as_slice))
    }
}

/// For an `IntersectSet` or a `DiffSet`, all result lines will be from the
/// first file operand, so we can avoid additional allocations by keeping its
/// text in memory and using subslices of its text as the members of the set.
///
/// For subsequent operands, we take a `SliceSet` `s` of the operand's text and
/// (for an `IntersectSet`) keep only those lines that occur in `s` or (for a
/// `DiffSet`) remove the lines that occur in `s`. We use a macro to avoid
/// two chunks of code differing in a single line.
macro_rules! impl_waning_set {
    ($WaningSet:ident, $filter:ident) => {
        impl<'data> LineSet<'data> for $WaningSet<'data> {
            fn insert_line(&mut self, line: &'data [u8]) {
                self.0.insert(line);
            }
        }
        impl<'data> SetExpression for $WaningSet<'data> {
            /// Remove (for `DiffSet`) or retain (for `IntersectSet`) the elements
            /// of `other`
            fn operate(&mut self, other: &[u8]) {
                let other = SliceSet::with(other);
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

#[allow(clippy::pedantic)]
#[cfg(test)]
mod test {
    use super::*;

    fn calc(operation: OpName, operands: &[&[u8]]) -> Vec<u8> {
        let mut answer = Vec::<u8>::new();
        let operands = operands.iter().map(|s| Ok(s.to_vec()));
        do_calculation(operation, operands, {
            |iter| {
                answer = iter.map(|s| s.to_owned()).flatten().collect();
                Ok(())
            }
        })
        .unwrap();
        answer
    }

    use self::OpName::*;

    #[test]
    fn given_a_single_argument_all_ops_but_multiple_return_its_lines_in_order_without_dups() {
        let arg: Vec<&[u8]> = vec![b"xxx\nabc\nxxx\nyyy\nxxx\nabc\n"];
        let uniq = b"xxx\nabc\nyyy\n";
        for op in &[Intersect, Union, Diff, Single, Multiple] {
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

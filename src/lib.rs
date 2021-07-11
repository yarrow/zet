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
#![allow(clippy::needless_return)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![deny(missing_docs)]

use anyhow::Result;
use std::borrow::Cow;
use std::vec::Vec;

use indexmap::{IndexMap, IndexSet};

pub mod args;
use crate::args::OpName;
pub mod io;
use crate::io::lines_of;

/// The `LineIterator` type is used to return the value of a `SetExpression` `s`:
/// `s.iter()` returns an iterator over the lines (elements) of `s`.
///
pub(crate) type LineIterator<'a> = Box<dyn Iterator<Item = &'a [u8]> + 'a>;

// A `SliceSet` is a set of slices borrowed from a text string, each slice
// corresponding to a line.
//
type SliceSet<'data> = IndexSet<&'data [u8]>;

fn slice_set(operand: &[u8]) -> SliceSet {
    let mut set = SliceSet::default();
    for line in lines_of(operand) {
        set.insert(line);
    }
    set
}

// The members of a `UnionSet` are borrowed if they come from the first file
// argument and owned otherwise. If the files whose lines we're taking the union
// of are substantially identical, we'll use memory roughly equal to the size of
// the first file. If most of the lines come from the second and subsequent files,
// then we don't gain much, but we don't lose much either.
//
type UnionSet<'data> = IndexSet<Cow<'data, [u8]>>;

// A `CountedSet` must keep track of whether its members were found in just
// one file, or in multiple files. After processing all files, we return
// for OpName::Single the lines found in just one file, and for
// OpName::Multiple the lines found in more than one file.
//
type CountedSet<'data> = IndexMap<Cow<'data, [u8]>, FoundIn>;

#[derive(PartialEq)]
enum FoundIn {
    One,
    Many,
}

/// Calculates and prints the set operation named by `op`. Each file in `files`
/// is treated as a set of lines:
///
/// * `OpName::Union` prints the lines that occur in any file,
/// * `OpName::Intersect` prints the lines that occur in all files,
/// * `OpName::Diff` prints the lines that occur in the first file and no other,
/// * `OpName::Single` prints the lines that occur in exactly one file, and
/// * `OpName::Multiple` prints the lines that occur in more than one file.
///
pub fn do_calculation(
    operation: OpName,
    first_operand: &[u8],
    rest: impl IntoIterator<Item = Result<Vec<u8>>>,
    output: impl FnOnce(LineIterator) -> Result<()>,
) -> Result<()> {
    let rest = rest.into_iter();

    match operation {
        OpName::Union => {
            let mut set = UnionSet::default();
            for line in lines_of(first_operand) {
                set.insert(Cow::Borrowed(line));
            }
            for operand in rest {
                for line in lines_of(&operand?) {
                    set.insert(Cow::from(line.to_vec()));
                }
            }
            return output(Box::new(set.iter().map(Cow::as_ref)));
        }

        OpName::Intersect | OpName::Diff => {
            // Note: IndexSet's `retain` method keeps the order of the retained
            // elements, but `remove` does not. So we can't just remove elements one by
            // one when they're not wanted. We'll execute the order(n) `retain` operation
            // `f - 1` times, where `f` is the number of files we examine.
            //
            let mut set = slice_set(first_operand);
            for operand in rest {
                // I don't know why this has to be two statements, but the borrow
                // checker hates us if we just use `slice_set(&operand?)`
                let operand = operand?;
                let other = slice_set(&operand);
                if operation == OpName::Intersect {
                    set.retain(|x| other.contains(x))
                } else {
                    set.retain(|x| !other.contains(x))
                }
            }
            return output(Box::new(set.iter().copied()));
        }

        OpName::Single | OpName::Multiple => {
            let mut set = CountedSet::default();
            for line in lines_of(first_operand) {
                set.insert(Cow::Borrowed(line), FoundIn::One);
            }
            for operand in rest {
                let operand = operand?;
                let other = slice_set(&operand);
                for line in other.iter() {
                    match set.get_mut(*line) {
                        None => {
                            set.insert(Cow::from(line.to_vec()), FoundIn::One);
                        }
                        Some(v) => *v = FoundIn::Many,
                    }
                }
            }
            let wanted = if operation == OpName::Single { FoundIn::One } else { FoundIn::Many };
            set.retain(|_k, v| *v == wanted);
            return output(Box::new(set.keys().map(Cow::as_ref)));
        }
    };
}

#[allow(clippy::pedantic)]
#[cfg(test)]
mod test {
    use super::*;

    fn calc(operation: OpName, operands: &[&[u8]]) -> Vec<u8> {
        fn add_eol(s: &[u8]) -> Vec<u8> {
            let mut s = s.to_owned();
            s.push(b'\n');
            s
        }
        let mut answer = Vec::<u8>::new();
        let mut operands = operands.iter().map(|s| Ok(s.to_vec()));
        let first = operands.next().unwrap().unwrap();
        do_calculation(operation, &first, operands, {
            |iter| {
                answer = iter.map(|s| add_eol(s)).flatten().collect();
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
        let uniq = b"xxx\nabc\nyyy\n".to_vec();
        let empty = b"".to_vec();
        for op in &[Intersect, Union, Diff, Single, Multiple] {
            let result = calc(*op, &arg);
            let expected = if *op == Multiple { &empty } else { &uniq };
            assert_eq!(result, *expected, "for {:?}", op);
        }
    }
    #[test]
    fn results_for_each_operation() {
        let args: Vec<&[u8]> = vec![
            b"xyz\nabc\nxy\nxz\nx\n", // Strings containing "x" (and "abc")
            b"xyz\nabc\nxy\nyz\ny\n", // Strings containing "y" (and "abc")
            b"xyz\nabc\nxz\nyz\nz\n", // Strings containing "z" (and "abc")
        ];
        assert_eq!(calc(Union, &args), b"xyz\nabc\nxy\nxz\nx\nyz\ny\nz\n", "for {:?}", Union);
        assert_eq!(calc(Intersect, &args), b"xyz\nabc\n", "for {:?}", Intersect);
        assert_eq!(calc(Diff, &args), b"x\n", "for {:?}", Diff);
        assert_eq!(calc(Single, &args), b"x\ny\nz\n", "for {:?}", Single);
        assert_eq!(calc(Multiple, &args), b"xyz\nabc\nxy\nxz\nyz\n", "for {:?}", Multiple);
    }
}

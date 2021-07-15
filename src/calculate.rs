//! Houses the `exec` function
//!
use anyhow::Result;
use std::borrow::Cow;
use std::vec::Vec;

use crate::args::OpName;
use crate::io::lines_of;
use crate::LineIterator;
use fxhash::FxBuildHasher;
use indexmap::IndexMap;

type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;
fn borrow_from<Bookkeeping: Copy>(operand: &[u8], b: Bookkeeping) -> CowSet<Bookkeeping> {
    let mut set = CowSet::default();
    for line in lines_of(operand) {
        set.insert(Cow::Borrowed(line), b);
    }
    set
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
pub fn exec(
    operation: OpName,
    first_operand: &[u8],
    rest: impl IntoIterator<Item = Result<Vec<u8>>>,
    output: impl FnOnce(LineIterator) -> Result<()>,
) -> Result<()> {
    let rest = rest.into_iter();

    match operation {
        OpName::Union => {
            let mut set = borrow_from(first_operand, ());
            for operand in rest {
                for line in lines_of(&operand?) {
                    set.insert(Cow::from(line.to_vec()), ());
                }
            }
            return output(Box::new(set.keys().map(Cow::as_ref)));
        }

        OpName::Diff => {
            let mut set = borrow_from(first_operand, true);
            for operand in rest {
                for line in lines_of(&operand?) {
                    if let Some(keepme) = set.get_mut(line) {
                        *keepme = false;
                    }
                }
            }
            set.retain(|_k, keepme| *keepme);
            return output(Box::new(set.keys().map(Cow::as_ref)));
        }

        OpName::Intersect => {
            const BLUE: bool = true; //  We're using Booleans, but we could
            const _RED: bool = false; // be using two different colors
            let mut this_cycle = BLUE;
            let mut set = borrow_from(first_operand, this_cycle);
            for operand in rest {
                this_cycle = !this_cycle; // flip BLUE -> RED and RED -> BLUE
                for line in lines_of(&operand?) {
                    if let Some(when_seen) = set.get_mut(line) {
                        *when_seen = this_cycle;
                    }
                }
                set.retain(|_k, when_seen| *when_seen == this_cycle);
            }
            return output(Box::new(set.keys().map(Cow::as_ref)));
        }

        OpName::Single | OpName::Multiple => {
            #[derive(Clone, Copy)]
            struct SeenIn {
                first: u32,
                last: u32,
            }
            let mut operand_count = 0_u32;
            let mut set = borrow_from(first_operand, SeenIn { first: 0_u32, last: 0_u32 });
            for operand in rest {
                if operand_count == std::u32::MAX {
                    anyhow::bail!("Can't handle more than {} arguments", std::u32::MAX);
                }
                operand_count = operand_count.wrapping_add(1);

                let seen_now = SeenIn { first: operand_count, last: operand_count };

                for line in lines_of(&operand?) {
                    match set.get_mut(line) {
                        None => {
                            set.insert(Cow::from(line.to_vec()), seen_now);
                        }
                        Some(seen_in) => seen_in.last = operand_count,
                    }
                }
            }
            if operation == OpName::Single {
                set.retain(|_k, seen_in| seen_in.first == seen_in.last);
            } else {
                set.retain(|_k, seen_in| seen_in.first != seen_in.last);
            }
            return output(Box::new(set.keys().map(Cow::as_ref)));
        }
    }
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
        exec(operation, &first, operands, {
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

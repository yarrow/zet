//! Houses the `calculate` function
//!
use std::num::NonZeroUsize;

use anyhow::Result;

use crate::args::OpName;
use crate::set::{Counted, LaterOperand, Tally, Uncounted, ZetSet};

/// Calculates and prints the set operation named by `op`. Each file in `files`
/// is treated as a set of lines:
///
/// * `OpName::Union` prints the lines that occur in any file,
/// * `OpName::Intersect` prints the lines that occur in all files,
/// * `OpName::Diff` prints the lines that occur in the first file and no other,
/// * `OpName::SingleByFile` prints the lines that occur in exactly one file, and
/// * `OpName::MultipleByFile` prints the lines that occur in more than one file.
///
pub fn calculate<O: LaterOperand>(
    operation: OpName,
    count_lines: bool,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    if count_lines {
        inner(operation, Counted::new(), first_operand, rest, out)
    } else {
        inner(operation, Uncounted::new(), first_operand, rest, out)
    }
}
fn output<SetTally: Tally, PrintTally: Tally, Item: Copy>(
    set: &ZetSet<Item, SetTally>,
    maybe_count: PrintTally,
    out: impl std::io::Write,
) -> Result<()> {
    if maybe_count.actually_counts() {
        set.output_with_count_to(out)
    } else {
        set.output_to(out)
    }
}

fn inner<O: LaterOperand, Counter: Tally>(
    operation: OpName,
    count: Counter,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    fn union<O: LaterOperand, Counter: Tally>(
        first_operand: &[u8],
        rest: impl Iterator<Item = Result<O>>,
        count: Counter,
    ) -> Result<ZetSet<(), Counter>> {
        let mut set = ZetSet::new(first_operand, (), count);
        for operand in rest {
            set.update(operand?, (), |_| {})?;
        }
        Ok(set)
    }
    match operation {
        // `Union` doesn't need bookkeeping, so we use the unit type as its
        // bookkeeping value.
        OpName::Union => {
            let set = union(first_operand, rest, count)?;
            output(&set, count, out)
        }

        // `Single` and `Multiple` print those lines that occur once and more than once,
        // respectively, in the entire input.
        OpName::Single | OpName::Multiple => {
            let mut set = union(first_operand, rest, Counted::new())?;

            if operation == OpName::Single {
                set.retain_single();
            } else {
                set.retain_multiple();
            }

            output(&set, count, out)
        }

        // For `Diff`, the bookkeeping value of `true` means we've seen the line
        // only in the first operand, and `false` that the line is present in
        // some other operand.
        OpName::Diff => {
            let mut set = ZetSet::new(first_operand, true, count);
            for operand in rest {
                set.modify_if_present(operand?, |keepme| *keepme = false)?;
            }
            set.retain(|keepme| keepme);
            output(&set, count, out)
        }

        // `Intersect` is more complicated — we start with each line in the
        // first operand colored with `this_cycle`. So
        // (1)  All lines in `set` colored with `this_cycle` have been seen in
        //      every operand so far, and
        // (2)  All lines in `set` are colored with `this_cycle`, so
        // (3)  All lines in `set` have been seen in every operand so far.
        //
        // When we look at the next operand, (1) becomes unknown. We restore its
        // truth by
        // * Flipping `this_cycle` to the opposite color.
        // * Setting every line that occurs in the the next operand to the new
        //   value of `this_cycle`.
        // Then we restore the truth of (2) by removing every line whose
        // bookkeeping value is not `this_cycle`
        //
        // Once we've done this for each operand, the remaining lines are those
        // occurring in each operand, so we've calculated the intersection of
        // the operands.
        OpName::Intersect => {
            const BLUE: bool = true; //  We're using Booleans, but we could
            const _RED: bool = false; // be using two different colors

            let mut set = ZetSet::new(first_operand, BLUE, count);
            let mut this_cycle = BLUE;
            for operand in rest {
                this_cycle = !this_cycle; // flip BLUE -> RED and RED -> BLUE
                set.modify_if_present(operand?, |when_seen| *when_seen = this_cycle)?;
                set.retain(|when_seen| when_seen == this_cycle);
            }
            output(&set, count, out)
        }

        // For `SingleByFile` and `MultipleByFile`, we keep track of the id number of the
        // operand in which each line occurs, if there is exactly one such
        // operand. At the end, if a line has occurred in just one operand,
        // with id n, then its bookkeeping value will be Some(n).  If it occurs
        // in multiple operands, then its bookkeeping value will be None.
        // At the end:
        // *  For `SingleByFile`, we keep the operands with Some(n)
        // *  For `MultipleByFile`, we keep the operands with None (meaning the line was
        //    seen in at least two operands).:
        // As you may have noticed, at the end we don't care *what* the id n is,
        // just that there is only one.  We keep track of n because a line that
        // occurs multiple times, but only in a single operand, is still
        // considered to have occurred once.
        OpName::SingleByFile | OpName::MultipleByFile => {
            let seen_in_first_operand = NonZeroUsize::new(1);
            let mut this_operand_uid = seen_in_first_operand.expect("1 is nonzero");
            let mut set = ZetSet::new(first_operand, seen_in_first_operand, count);

            for operand in rest {
                let seen_in_this_operand = this_operand_uid.checked_add(1);
                match seen_in_this_operand {
                    Some(n) => this_operand_uid = n,
                    None => anyhow::bail!("Can't handle {} arguments", std::usize::MAX),
                }
                set.update(operand?, seen_in_this_operand, |unique_source| {
                    if *unique_source != seen_in_this_operand {
                        *unique_source = None
                    }
                })?;
            }

            if operation == OpName::SingleByFile {
                set.retain(|unique_source| unique_source.is_some());
            } else {
                set.retain(|unique_source| unique_source.is_none());
            }

            output(&set, count, out)
        }
    }
}

#[allow(clippy::pedantic)]
#[cfg(test)]
mod test {
    use super::*;
    use crate::operands;
    use assert_fs::{prelude::*, TempDir};
    use bstr::ByteSlice;
    use std::path::PathBuf;

    fn calc(operation: OpName, operands: &[&[u8]]) -> String {
        let first = operands[0];
        let remaining = operands[1..].iter().map(|s| s.to_vec());

        let temp_dir = TempDir::new().unwrap();
        let mut paths = Vec::new();
        for operand in remaining {
            let name = format!("operand{}", paths.len());
            let op = temp_dir.child(name);
            op.write_binary(&operand[..]).unwrap();
            paths.push(PathBuf::from(op.path()));
        }

        let mut answer = Vec::new();
        calculate(operation, false, first, operands::Remaining::from(paths), &mut answer).unwrap();
        let slow = String::from_utf8(answer).unwrap();
        let fast = fast_calc(operation, operands);
        assert_eq!(slow, fast);
        slow
    }

    // Like `calc`, but does no disk I/O
    fn fast_calc(operation: OpName, operands: &[&[u8]]) -> String {
        let first = operands[0];
        let mut answer = Vec::new();
        let rest = operands[1..].iter().map(|o| Ok(*o));
        calculate(operation, false, first, rest, &mut answer).unwrap();
        String::from_utf8(answer).unwrap()
    }
    impl LaterOperand for &[u8] {
        fn for_byte_line(self, for_each_line: impl FnMut(&[u8])) -> Result<()> {
            self.lines().for_each(for_each_line);
            Ok(())
        }
    }

    use self::OpName::*;

    #[test]
    fn given_a_single_argument_all_ops_but_multiple_return_its_lines_in_order_without_dups() {
        let arg: Vec<&[u8]> = vec![b"xxx\nabc\nxxx\nyyy\nxxx\nabc\n"];
        let uniq = "xxx\nabc\nyyy\n";
        let empty = "";
        for op in &[Intersect, Union, Diff, SingleByFile, MultipleByFile] {
            let result = calc(*op, &arg);
            let expected = if *op == MultipleByFile { empty } else { uniq };
            assert_eq!(result, *expected, "for {op:?}");
        }
    }
    #[test]
    fn results_for_each_operation() {
        let args: Vec<&[u8]> = vec![
            b"xyz\nabc\nxy\nxz\nx\n", // Strings containing "x" (and "abc")
            b"xyz\nabc\nxy\nyz\ny\n", // Strings containing "y" (and "abc")
            b"xyz\nabc\nxz\nyz\nz\n", // Strings containing "z" (and "abc")
        ];
        assert_eq!(calc(Union, &args), "xyz\nabc\nxy\nxz\nx\nyz\ny\nz\n", "for {Union:?}");
        assert_eq!(calc(Intersect, &args), "xyz\nabc\n", "for {Intersect:?}");
        assert_eq!(calc(Diff, &args), "x\n", "for {Diff:?}");
        assert_eq!(calc(SingleByFile, &args), "x\ny\nz\n", "for {SingleByFile:?}");
        assert_eq!(calc(MultipleByFile, &args), "xyz\nabc\nxy\nxz\nyz\n", "for {MultipleByFile:?}");
    }
}

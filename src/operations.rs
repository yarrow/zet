//! Houses the `calculate` function
//!
use std::num::NonZeroUsize;

use anyhow::Result;

use crate::args::OpName;
use crate::set::ToZetSet;

/// The `calculate` function's only requirement for its second and succeeding
/// operands is that they implement `for_byte_line`. The `LaterOperand` trait
/// codifies that.
pub trait LaterOperand {
    /// The call `o.for_byte_line(|line| ...)` method calls a the given closure
    /// for each &[u8] in `o`.
    fn for_byte_line(self, for_each_line: impl FnMut(&[u8])) -> Result<()>;
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
pub fn calculate<O: LaterOperand>(
    operation: OpName,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    match operation {
        // `Union` doesn't need bookkeeping, so we use the unit type as its
        // bookkeeping value.
        OpName::Union => {
            let mut set = first_operand.to_zet_set_with(());
            for operand in rest {
                operand?.for_byte_line(|line| {
                    set.insert(line, ());
                })?;
            }
            return set.output_to(out);
        }

        // For `Diff`, the bookkeeping value of `true` means we've seen the line
        // only in the first operand, and `false` that the line is present in
        // some other operand.
        OpName::Diff => {
            let mut set = first_operand.to_zet_set_with(true);
            for operand in rest {
                operand?.for_byte_line(|line| {
                    if let Some(keepme) = set.get_mut(line) {
                        *keepme = false;
                    }
                })?;
            }
            set.retain(|keepme| *keepme);
            return set.output_to(out);
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
            let mut set = first_operand.to_zet_set_with(BLUE);
            let mut this_cycle = BLUE;
            for operand in rest {
                this_cycle = !this_cycle; // flip BLUE -> RED and RED -> BLUE
                operand?.for_byte_line(|line| {
                    if let Some(when_seen) = set.get_mut(line) {
                        *when_seen = this_cycle;
                    }
                })?;
                set.retain(|when_seen| *when_seen == this_cycle);
            }
            return set.output_to(out);
        }

        // For `Single` and `Multiple`, we keep track of the id number of the
        // operand in which each line occurs, if there is exactly one such
        // operand. At the end, if a line has occurred in just one operand,
        // with id n, then its bookkeeping value will be Some(n).  If it occurs
        // in multiple operands, then its bookkeeping value will be None.
        // At the end:
        // *  For `Single`, we keep the operands with Some(n)
        // *  For `Multiple`, we keep the operands with None (meaning the line was
        //    seen in at least two operands).:
        // As you may have noticed, at the end we don't care *what* the id n is,
        // just that there is only one.  We keep track of n because a line that
        // occurs multiple times, but only in a single operand, is still
        // considered to have occurred once.
        OpName::Single | OpName::Multiple => {
            let seen_in_first_operand = NonZeroUsize::new(1);
            let mut this_operand_uid = seen_in_first_operand.expect("1 is nonzero");
            let mut set = first_operand.to_zet_set_with(seen_in_first_operand);

            for operand in rest {
                let seen_in_this_operand = this_operand_uid.checked_add(1);
                match seen_in_this_operand {
                    Some(n) => this_operand_uid = n,
                    None => anyhow::bail!("Can't handle {} arguments", std::usize::MAX),
                }
                operand?.for_byte_line(|line| match set.get_mut(line) {
                    None => set.insert(line, seen_in_this_operand),
                    Some(unique_source) => {
                        if *unique_source != seen_in_this_operand {
                            *unique_source = None;
                        }
                    }
                })?;
            }

            if operation == OpName::Single {
                set.retain(|unique_source| unique_source.is_some());
            } else {
                set.retain(|unique_source| unique_source.is_none());
            }

            return set.output_to(out);
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
        calculate(operation, first, operands::Remaining::from(paths), &mut answer).unwrap();
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
        calculate(operation, first, rest, &mut answer).unwrap();
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
        for op in &[Intersect, Union, Diff, Single, Multiple] {
            let result = calc(*op, &arg);
            let expected = if *op == Multiple { empty } else { uniq };
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
        assert_eq!(calc(Single, &args), "x\ny\nz\n", "for {Single:?}");
        assert_eq!(calc(Multiple, &args), "xyz\nabc\nxy\nxz\nyz\n", "for {Multiple:?}");
    }
}

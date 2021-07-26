//! Houses the `calculate` function
//!
use anyhow::Result;

use crate::args::OpName;
use crate::operands;
use crate::set::ToZetSet;

/// Calculates and prints the set operation named by `op`. Each file in `files`
/// is treated as a set of lines:
///
/// * `OpName::Union` prints the lines that occur in any file,
/// * `OpName::Intersect` prints the lines that occur in all files,
/// * `OpName::Diff` prints the lines that occur in the first file and no other,
/// * `OpName::Single` prints the lines that occur in exactly one file, and
/// * `OpName::Multiple` prints the lines that occur in more than one file.
///
pub fn calculate(
    operation: OpName,
    first_operand: &[u8],
    rest: operands::Remaining,
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
                })?
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
                })?
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

        // For `Single` and `Multiple`, we keep track of the first and the last
        // operand in which each line occurs.
        // At the end,
        // *  For `Single`, we keep the opertands for which `first == last`
        // *  For `Multiple`, we keep the opertands for which `first != last`
        //    (so the line was seen in at least two operands).
        OpName::Single | OpName::Multiple => {
            #[derive(Clone, Copy)]
            struct SeenIn {
                first: u32,
                last: u32,
            }

            let mut operand_count = 0_u32;
            let mut set = first_operand.to_zet_set_with(SeenIn { first: 0_u32, last: 0_u32 });

            for operand in rest {
                if operand_count == std::u32::MAX {
                    anyhow::bail!("Can't handle more than {} arguments", std::u32::MAX);
                }
                operand_count += 1;

                let seen_now = SeenIn { first: operand_count, last: operand_count };
                operand?.for_byte_line(|line| match set.get_mut(line) {
                    None => {
                        set.insert(line, seen_now);
                    }
                    Some(seen_in) => seen_in.last = operand_count,
                })?
            }

            if operation == OpName::Single {
                set.retain(|seen_in| seen_in.first == seen_in.last);
            } else {
                set.retain(|seen_in| seen_in.first != seen_in.last);
            }

            return set.output_to(out);
        }
    }
}

#[allow(clippy::pedantic)]
#[cfg(test)]
mod test {
    use super::*;
    use assert_fs::{prelude::*, TempDir};
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
        String::from_utf8(answer).unwrap()
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
        assert_eq!(calc(Union, &args), "xyz\nabc\nxy\nxz\nx\nyz\ny\nz\n", "for {:?}", Union);
        assert_eq!(calc(Intersect, &args), "xyz\nabc\n", "for {:?}", Intersect);
        assert_eq!(calc(Diff, &args), "x\n", "for {:?}", Diff);
        assert_eq!(calc(Single, &args), "x\ny\nz\n", "for {:?}", Single);
        assert_eq!(calc(Multiple, &args), "xyz\nabc\nxy\nxz\nyz\n", "for {:?}", Multiple);
    }
}

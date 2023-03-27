//! Houses the `calculate` function
//!
use anyhow::Result;

use crate::args::OpName::{
    self, Diff, Intersect, Multiple, MultipleByFile, Single, SingleByFile, Union,
};
use crate::set::{LaterOperand, ZetSet};
use crate::tally::{Bookkeeping, Dual, FileCount, LastFileSeen, LineCount, Noop, Select};

#[derive(Clone, Copy)]
pub enum Count {
    Lines,
    Files,
    Nothing,
}
/// Calculates and prints the set operation named by `operation`. Each file in `files`
/// is treated as a set of lines:
///
/// * `OpName::Union` prints the lines that occur in any file,
/// * `OpName::Intersect` prints the lines that occur in all files,
/// * `OpName::Diff` prints the lines that occur in the first file and no other,
/// * `OpName::Single` prints the lines that occur once in exactly in the input,
/// * `OpName::Multiple` prints the lines that occur more than once in the input,
/// * `OpName::SingleByFile` prints the lines that occur in exactly one file, and
/// * `OpName::MultipleByFile` prints the lines that occur in more than one file.
///
pub fn calculate<O: LaterOperand>(
    operation: OpName,
    count: bool,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    if count {
        calculate2(operation, Count::Lines, first_operand, rest, out)
    } else {
        calculate2(operation, Count::Nothing, first_operand, rest, out)
    }
}

pub fn calculate2<O: LaterOperand>(
    operation: OpName,
    count: Count,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    match count {
        Count::Nothing => dispatch(operation, Noop::first_file(), first_operand, rest, out),

        // When `count` is `Count::Lines` and `operation` is `Single` or
        // `Multiple`, both logging and selection need a `LineCount` in the
        // bookkeeping item, so `dispatch` would call `count_and` with
        // bookkeeping values of `Dual<LineCount, LineCount>`. It would be safe
        // to count each line in both fields of a `Dual` item, but slower.  And
        // it seems unlikely that the optimizer would avoid doing the counting
        // twice. So we call `count_and` directly, with a single `LineCount`
        // bookkeeping value.
        Count::Lines => match operation {
            Single => count_and::<LineCount, O>(Keep::Single, first_operand, rest, out),
            Multiple => count_and::<LineCount, O>(Keep::Multiple, first_operand, rest, out),
            _ => dispatch(operation, LineCount::first_file(), first_operand, rest, out),
        },

        // Similarly, we don't want `dispatch` to use `Dual<FileCount, FileCount>`
        // bookkeeping values, so we call `count_and` directly when `count` is
        // Count::Files` and `operation` is `SingleByFile` or `MultipleByFile`.
        Count::Files => match operation {
            SingleByFile => count_and::<FileCount, O>(Keep::Single, first_operand, rest, out),
            MultipleByFile => count_and::<FileCount, O>(Keep::Multiple, first_operand, rest, out),

            _ => dispatch(operation, FileCount::first_file(), first_operand, rest, out),
        },
    }
}

/// The `dispatch` function calls the relevant function to do the actual work.
/// Calling `dispatch` from `calculate` means that the monomorphizer knows the
/// type of `log`, and create three different versions of `dispatch`, for `Noop`, `LineCount`,
/// and `FileCount` (and so three different version of `union`, `diff`,
/// `intersect`, `count_lines_and`, and `count_files_and`).
fn dispatch<Log: Bookkeeping, O: LaterOperand>(
    operation: OpName,
    log: Log,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    type LineAnd<Log> = Dual<LineCount, Log>;
    type FileAnd<Log> = Dual<FileCount, Log>;
    match operation {
        Union => union(log, first_operand, rest, out),
        Diff => diff(log, first_operand, rest, out),
        Intersect => intersect(log, first_operand, rest, out),
        Single => count_and::<LineAnd<Log>, O>(Keep::Single, first_operand, rest, out),
        Multiple => count_and::<LineAnd<Log>, O>(Keep::Multiple, first_operand, rest, out),
        SingleByFile => count_and::<FileAnd<Log>, O>(Keep::Single, first_operand, rest, out),
        MultipleByFile => count_and::<FileAnd<Log>, O>(Keep::Multiple, first_operand, rest, out),
    }
}

/// For most operations, we insert every line in the input into the `ZetSet`.
/// Both `new` and `insert_or_modify` will call `item.modify(file_number)` on
/// the line's bookkeeping item if the line is already present in the `ZetSet`.
/// The operation will then call `set.retain()` to examine the each line's
/// bookkeeping item to decide whether or not it belongs in the set.
fn every_line<O: LaterOperand, B: Bookkeeping>(
    mut item: B,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
) -> Result<ZetSet<B>> {
    assert_eq!(item, B::first_file());
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file();
        set.insert_or_modify(operand?, item)?;
    }
    Ok(set)
}

/// `Union` collects every line, so we don't need to call `retain`; and
/// the only bookkeeping needed is for the line/file counts, so we don't
/// need a `Dual` bookkeeping value and just use the `Log` argument passed in.
fn union<Log: Bookkeeping, O: LaterOperand>(
    log: Log,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    assert_eq!(log, Log::first_file());
    let set = every_line(log, first_operand, rest)?;
    output_and_discard(set, out)
}

/// Only lines that appear in the first operand will be in the result of `Diff`;
/// so `Diff` uses `modify_if_present` rather than `insert_or_modify`, changing
/// the file number of each file seen in a subsequent operand. We discard lines
/// whose `LastFileSeen` value is not `1`, so we're left only with lines that
/// appear only in the first file.
fn diff<Log: Bookkeeping, O: LaterOperand>(
    _log: Log,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut item = Dual::<LastFileSeen, Log>::first_file();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file();
        set.modify_if_present(operand?, item.file_number())?;
    }
    set.retain(|v| v == 1);
    output_and_discard(set, out)
}

/// Similarly, only lines that appear in the first operand will be in the result
/// of `Intersect`; so `Intersect` also uses `modify_if_present` rather than
/// `insert_or_modify`. But lines in `Intersect`'s result must also appear in
/// every other file; so after each file we discard those lines whose
/// `LastFileSeen` number is not the current `file_number`.
fn intersect<Log: Bookkeeping, O: LaterOperand>(
    _log: Log,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut item = Dual::<LastFileSeen, Log>::first_file();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file();
        set.modify_if_present(operand?, item.file_number())?;
        set.retain(|v| v == item.file_number());
    }
    output_and_discard(set, out)
}

/// For `Single` and `Multiple` each line's `LineCount` item will keep track of
/// how many times it has appeared in the entire input.  For `SingleByFile` and
/// `MultipleByFile` each line's bookkeeping item will keep track of how many
/// files the line has appeared in.
///
/// For `Single` and `SingleByFile` we'll call `count_and(Keep::Single, ...)`
/// and for `Multiple` and `MultipleByFile` we'll call `count_and(Keep:Multiple, ...)`
#[derive(Clone, Copy, PartialEq)]
enum Keep {
    Single,
    Multiple,
}

/// Create a `ZetSet` whose bookkeeping items must keep track of the number of
/// times a line has appeared in the input, or the number of files it has
/// appeared in.  Then retain those whose bookkeeping item's value is 1 (for
/// `Keep::Single`) or greater than 1 (for `Keep::Multiple`).
fn count_and<B: Bookkeeping, O: LaterOperand>(
    keep: Keep,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let item = B::first_file();
    let mut set = every_line(item, first_operand, rest)?;
    match keep {
        Keep::Single => set.retain(|v| v == 1),
        Keep::Multiple => set.retain(|v| v > 1),
    }
    output_and_discard(set, out)
}

/// When we're done with a `ZetSet`, we write its lines to our output and exit
/// the program.
fn output_and_discard<B: Bookkeeping>(set: ZetSet<B>, out: impl std::io::Write) -> Result<()> {
    set.output_to(out)?;
    std::mem::forget(set); // Slightly faster to just abandon this, since we're about to exit.
                           // Thanks to [Karolin Varner](https://github.com/koraa)'s huniq
    Ok(())
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

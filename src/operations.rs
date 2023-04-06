//! Houses the `calculate` function
//!
use anyhow::{bail, Result};
use std::fmt::Debug;

use crate::args::OpName::{
    self, Diff, Intersect, Multiple, MultipleByFile, Single, SingleByFile, Union,
};
use crate::set::{LaterOperand, ZetSet};

#[derive(Clone, Copy, Debug)]
pub enum LogType {
    Lines,
    Files,
    None,
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
/// The `log_type` operand specifies whether `calculate` should print the number
/// of time each line appears in the input (`LogType::Lines`), the number of
/// files in which each argument appears (`LogType::Files`), or neither
/// (`LogType::None`).
///
pub fn calculate<O: LaterOperand>(
    operation: OpName,
    log_type: LogType,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    match log_type {
        LogType::None => match operation {
            Union => union::<Unlogged<Noop>, O>(first_operand, rest, out),
            Diff => diff::<Unlogged<LastFileSeen>, O>(first_operand, rest, out),
            Intersect => intersect::<Unlogged<LastFileSeen>, O>(first_operand, rest, out),
            Single => count::<Unlogged<LineCount>, O>(AndKeep::Single, first_operand, rest, out),
            Multiple => {
                count::<Unlogged<LineCount>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
            SingleByFile => {
                count::<Unlogged<FileCount>, O>(AndKeep::Single, first_operand, rest, out)
            }
            MultipleByFile => {
                count::<Unlogged<FileCount>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
        },

        // When `log_type` is `LogType::Lines` and `operation` is `Single` or
        // `Multiple`, both logging and selection need a `LineCount` in the
        // bookkeeping item, so we only need `Logged<LineCount>`, not
        // bookkeeping values of `Dual<LineCount, LineCount>`.
        LogType::Lines => match operation {
            Union => union::<Dual<Noop, LineCount>, O>(first_operand, rest, out),
            Diff => diff::<Dual<LastFileSeen, LineCount>, O>(first_operand, rest, out),
            Intersect => intersect::<Dual<LastFileSeen, LineCount>, O>(first_operand, rest, out),
            Single => count::<Logged<LineCount>, O>(AndKeep::Single, first_operand, rest, out),
            Multiple => count::<Logged<LineCount>, O>(AndKeep::Multiple, first_operand, rest, out),
            SingleByFile => {
                count::<Dual<FileCount, LineCount>, O>(AndKeep::Single, first_operand, rest, out)
            }
            MultipleByFile => {
                count::<Dual<FileCount, LineCount>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
        },

        // Similarly, we don't want to use `Dual<FileCount, FileCount>`
        // bookkeeping values, so we use `Logged<FileCount>` when `log_type` is
        // LogType::Files` and `operation` is `SingleByFile` or
        // `MultipleByFile`.
        //
        // And we use `Logged<LineCount>` for `Single`, rather than
        // `Dual<LineCount, FileCount>`, since the number reported for `Single`
        // will always be 1 — a line appearing only once can appear in only one
        // file.
        LogType::Files => match operation {
            Union => union::<Dual<Noop, FileCount>, O>(first_operand, rest, out),
            Diff => diff::<Dual<LastFileSeen, FileCount>, O>(first_operand, rest, out),
            Intersect => intersect::<Dual<LastFileSeen, FileCount>, O>(first_operand, rest, out),
            Single => count::<Logged<LineCount>, O>(AndKeep::Single, first_operand, rest, out),
            Multiple => {
                count::<Dual<LineCount, FileCount>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
            SingleByFile => {
                count::<Logged<FileCount>, O>(AndKeep::Single, first_operand, rest, out)
            }
            MultipleByFile => {
                count::<Logged<FileCount>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
        },
    }
}

/// The `Bookkeeping` trait specifies the kind of types that can
/// serve as the bookkeeping values for a `ZetSet`.
pub(crate) trait Bookkeeping: Copy + PartialEq + Debug {
    fn new() -> Self;
    fn next_file(&mut self) -> Result<()>;
    fn update_with(&mut self, other: Self);
    fn retention_value(self) -> u32;
    fn count(self) -> u32;
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()>;
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Logged<R: Bookkeeping>(R);
impl<R: Bookkeeping> Bookkeeping for Logged<R> {
    fn new() -> Self {
        Self(R::new())
    }
    fn next_file(&mut self) -> Result<()> {
        self.0.next_file()
    }
    fn update_with(&mut self, other: Self) {
        self.0.update_with(other.0)
    }
    fn retention_value(self) -> u32 {
        self.0.retention_value()
    }
    fn count(self) -> u32 {
        self.0.retention_value()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        if self.count() == u32::MAX {
            write!(out, " overflow  ")?
        } else {
            write!(out, "{:width$} ", self.count())?
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Unlogged<R: Bookkeeping>(R);
impl<R: Bookkeeping> Bookkeeping for Unlogged<R> {
    fn new() -> Self {
        Self(R::new())
    }
    fn next_file(&mut self) -> Result<()> {
        self.0.next_file()
    }
    fn update_with(&mut self, other: Self) {
        self.0.update_with(other.0)
    }
    fn retention_value(self) -> u32 {
        self.0.retention_value()
    }
    /*
    fn output_zet_set(&set: ZetSet<R>, mut out: impl io::Write) -> Result<()> {
        out.write_all(set.bom)?;
        for line in set.keys {
            out.write_all(line)?;
            out.write_all(set.line_terminator)?;
        }
        out.flush()?;
        Ok(())
    }
    */
    fn count(self) -> u32 {
        0
    }
    fn write_count(&self, _width: usize, _out: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}

/// The `Dual` struct lets us use one item for retention purposes and another
/// for logging. We take the `retention_value` from the first item and `count`
/// and `write_count` from the second.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Dual<Retain: Bookkeeping, Log: Bookkeeping> {
    pub(crate) retention: Retain,
    pub(crate) log: Log,
}
impl<Retain: Bookkeeping, Log: Bookkeeping> Bookkeeping for Dual<Retain, Log> {
    fn new() -> Self {
        Dual { retention: Retain::new(), log: Log::new() }
    }
    fn next_file(&mut self) -> Result<()> {
        self.retention.next_file()?;
        self.log.next_file()
    }
    fn update_with(&mut self, other: Self) {
        self.retention.update_with(other.retention);
        self.log.update_with(other.log);
    }
    fn retention_value(self) -> u32 {
        self.retention.retention_value()
    }
    fn count(self) -> u32 {
        Logged(self.log).count()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        Logged(self.log).write_count(width, out)
    }
}

/// We use the `Noop` struct for the `Union` operation, since `Union` includes
/// every line seen and doesn't need bookkeeping. need to keep track of
/// anything. `Noop` is also used for the default log operantion of not logging
/// anything.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Noop();
impl Bookkeeping for Noop {
    fn new() -> Self {
        Noop()
    }
    fn next_file(&mut self) -> Result<()> {
        Ok(())
    }
    fn update_with(&mut self, _other: Self) {}
    fn retention_value(self) -> u32 {
        0
    }
    fn count(self) -> u32 {
        self.retention_value()
    }
    fn write_count(&self, _width: usize, _out: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}
/// For most operations, we insert every line in the input into the `ZetSet`.
/// Both `new` and `insert_or_update` will call `v.update_with(item)` on the
/// line's bookkeeping item `v` if the line is already present in the `ZetSet`.
/// The operation will then call `set.retain()` to examine the each line's
/// bookkeeping item to decide whether or not it belongs in the set.
fn every_line<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
) -> Result<ZetSet<B>> {
    let mut item = B::new();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file()?;
        set.insert_or_update(operand?, item)?;
    }
    Ok(set)
}

/// `Union` collects every line, so we don't need to call `retain`; and
/// the only bookkeeping needed is for the line/file counts, so we don't
/// need a `Dual` bookkeeping value and just use the `Log` argument passed in.
fn union<Log: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let set = every_line::<Log, O>(first_operand, rest)?;
    output_and_discard(set, out)
}

/// Only lines that appear in the first operand will be in the result of `Diff`;
/// so `Diff` uses `update_if_present` rather than `insert_or_update`, changing
/// the file number of each file seen in a subsequent operand. We discard lines
/// whose `LastFileSeen::retention_value` is not `1`, so we're left only with
/// lines that appear only in the first file.
fn diff<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut item = B::new();
    let first_file = item.retention_value();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file()?;
        set.update_if_present(operand?, item)?;
    }
    set.retain(|file_number| file_number == first_file);
    output_and_discard(set, out)
}

/// `LastFileSeen` is a thin wrapper around a `u32`, with `next_file` being a
/// checked increment
#[derive(Clone, Copy, PartialEq, Debug)]
struct LastFileSeen(u32);
impl Bookkeeping for LastFileSeen {
    fn new() -> Self {
        LastFileSeen(0)
    }
    fn next_file(&mut self) -> Result<()> {
        match self.0.checked_add(1) {
            Some(n) => self.0 = n,
            None => bail!("Zet can't handle more than {} input files", u32::MAX),
        }
        Ok(())
    }
    fn update_with(&mut self, other: Self) {
        self.0 = other.0
    }
    fn retention_value(self) -> u32 {
        self.0
    }
    fn count(self) -> u32 {
        0
    }
    fn write_count(&self, _width: usize, _out: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}
/// Similarly, only lines that appear in the first operand will be in the result
/// of `Intersect`; so `Intersect` as well as `Diff` uses `update_if_present`
/// rather than `insert_or_update`. But lines in `Intersect`'s result must also
/// appear in every other file; so after each file we discard those lines whose
/// `LastFileSeen` number is not the current `file_number`.
fn intersect<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut item = B::new();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file()?;
        let this_file = item.retention_value();
        set.update_if_present(operand?, item)?;
        set.retain(|last_file_seen| last_file_seen == this_file);
    }
    output_and_discard(set, out)
}

/// For `Single` and `Multiple` each line's `LineCount` item will keep track of
/// how many times it has appeared in the entire input. `LineCount` can also be
/// used for reporting the number of times each line appears in the input.
///
/// Like `LastFileSeen`, `LineCount` is a thin wrapper around `u32` — but
/// `LineCount` ignores `next_file`, and uses `update_with` only to increment the
/// `u32`. Here we use a saturating increment, because neither `Single` and
/// `Multiple` care only whether the `u32` is `1` or greater than `1`, and for
/// logging purposes it seems better to report overflow for lines that appear
/// `u32::MAX` times or more than to stop `zet` completely.
#[derive(Clone, Copy, PartialEq, Debug)]
struct LineCount(u32);
impl Bookkeeping for LineCount {
    fn new() -> Self {
        LineCount(1)
    }
    fn next_file(&mut self) -> Result<()> {
        Ok(())
    }
    fn update_with(&mut self, _other: Self) {
        self.0 = self.0.saturating_add(1);
    }
    fn retention_value(self) -> u32 {
        self.0
    }
    fn count(self) -> u32 {
        self.retention_value()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        if self.0 == u32::MAX {
            write!(out, " overflow  ")?
        } else {
            write!(out, "{:width$} ", self.0)?
        }
        Ok(())
    }
}

/// For `SingleByFile` and `MultipleByFile` each line's `FileCount` item will
/// keep track of how many files the line has appeared in. `FileCount` can also
/// be used to report the file count information for operatons whose selection
/// criteria are different from number of files.
///
/// Like `LastFileSeen`, `FileCount` keeps track of the last file seen, and
/// `bail`s if the number of files seen exceeds `u32::MAX`. It has a separate
/// `files_seen` field for tracking the number of files seen.
#[derive(Clone, Copy, PartialEq, Debug)]
struct FileCount {
    file_number: u32,
    files_seen: u32,
}
impl Bookkeeping for FileCount {
    fn new() -> Self {
        FileCount { file_number: 0, files_seen: 1 }
    }
    fn next_file(&mut self) -> Result<()> {
        match self.file_number.checked_add(1) {
            Some(n) => self.file_number = n,
            None => bail!("Zet can't handle more than {} input files", u32::MAX),
        }
        Ok(())
    }
    fn update_with(&mut self, other: Self) {
        if other.file_number != self.file_number {
            self.files_seen += 1;
            self.file_number = other.file_number;
        }
    }
    fn retention_value(self) -> u32 {
        self.files_seen
    }
    fn count(self) -> u32 {
        self.retention_value()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        write!(out, "{:width$} ", self.files_seen)?;
        Ok(())
    }
}

/// For `Single` and `SingleByFile` we'll call `count(AndKeep::Single, ...)`
/// and for `Multiple` and `MultipleByFile` we'll call `count(AndKeep:Multiple, ...)`
#[derive(Clone, Copy, PartialEq)]
enum AndKeep {
    Single,
    Multiple,
}

/// Create a `ZetSet` whose bookkeeping items must keep track of the number of
/// times a line has appeared in the input, or the number of files it has
/// appeared in.  Then retain those whose bookkeeping item's `retention_value`
/// is 1 (for `AndKeep::Single`) or greater than 1 (for `AndKeep::Multiple`).
fn count<B: Bookkeeping, O: LaterOperand>(
    keep: AndKeep,
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut set = every_line::<B, O>(first_operand, rest)?;
    match keep {
        AndKeep::Single => set.retain(|occurences| occurences == 1),
        AndKeep::Multiple => set.retain(|occurences| occurences > 1),
    }
    output_and_discard(set, out)
}

/// When we're done with a `ZetSet`, we write its lines to our output and exit
/// the program.
fn output_and_discard<B: Bookkeeping>(set: ZetSet<B>, out: impl std::io::Write) -> Result<()> {
    output_zet_set(&set, out)?;
    std::mem::forget(set); // Slightly faster to just abandon this, since we're about to exit.
                           // Thanks to [Karolin Varner](https://github.com/koraa)'s huniq
    Ok(())
}

/// Output the `ZetSet`'s lines with the appropriate Byte Order Mark and line
/// terminator.
fn output_zet_set<B: Bookkeeping>(set: &ZetSet<B>, mut out: impl std::io::Write) -> Result<()> {
    let Some(first) = set.first() else { return Ok(()) };

    if first.count() == 0 {
        // We're counting neither lines nor files
        out.write_all(set.bom)?;
        for line in set.keys() {
            out.write_all(line)?;
            out.write_all(set.line_terminator)?;
        }
        out.flush()?;
    } else {
        // We're counting something
        let Some(max_count) = set.values().map(|v| v.count()).max() else { return Ok(()) };
        let width = (max_count.ilog10() + 1) as usize;
        out.write_all(set.bom)?;
        for (line, item) in set.iter() {
            item.write_count(width, &mut out)?;
            out.write_all(line)?;
            out.write_all(set.line_terminator)?;
        }
        out.flush()?;
    };

    Ok(())
}
#[allow(clippy::pedantic)]
#[cfg(test)]
mod test {
    use super::*;
    use crate::operands;
    use bstr::ByteSlice;
    use indexmap::IndexMap;

    impl LaterOperand for &[u8] {
        fn for_byte_line(self, for_each_line: impl FnMut(&[u8])) -> Result<()> {
            self.lines().for_each(for_each_line);
            Ok(())
        }
    }

    type V8<'a> = [&'a [u8]];
    fn calc(operation: OpName, operands: &V8) -> String {
        let first = operands[0];
        let rest = operands[1..].iter().map(|o| Ok(*o));
        let mut answer = Vec::new();
        calculate(operation, LogType::None, first, rest, &mut answer).unwrap();
        String::from_utf8(answer).unwrap()
    }

    use self::OpName::*;

    #[test]
    fn given_a_single_argument_all_most_ops_return_input_lines_in_order_without_dups() {
        let arg: Vec<&[u8]> = vec![b"xxx\nabc\nxxx\nyyy\nxxx\nabc\n"];
        let uniq = "xxx\nabc\nyyy\n";
        let solo = "yyy\n";
        let multi = "xxx\nabc\n";
        let empty = "";
        for &op in &[Intersect, Union, Diff, Single, SingleByFile, Multiple, MultipleByFile] {
            let result = calc(op, &arg);
            let expected = if op == Single {
                solo
            } else if op == Multiple {
                multi
            } else if op == MultipleByFile {
                empty
            } else {
                uniq
            };
            assert_eq!(result, *expected, "for {op:?}");
        }
    }
    #[test]
    fn results_for_each_operation() {
        let args: Vec<&[u8]> = vec![
            b"xyz\nabc\nxy\nxz\nx\n",    // Strings containing "x" (and "abc")
            b"xyz\nabc\nxy\nyz\ny\ny\n", // Strings containing "y" (and "abc")
            b"xyz\nabc\nxz\nyz\nz\n",    // Strings containing "z" (and "abc")
        ];
        assert_eq!(calc(Union, &args), "xyz\nabc\nxy\nxz\nx\nyz\ny\nz\n", "for {Union:?}");
        assert_eq!(calc(Intersect, &args), "xyz\nabc\n", "for {Intersect:?}");
        assert_eq!(calc(Diff, &args), "x\n", "for {Diff:?}");
        assert_eq!(calc(Single, &args), "x\nz\n", "for {Single:?}");
        assert_eq!(calc(SingleByFile, &args), "x\ny\nz\n", "for {SingleByFile:?}");
        assert_eq!(calc(Multiple, &args), "xyz\nabc\nxy\nxz\nyz\ny\n", "for {Multiple:?}");
        assert_eq!(calc(MultipleByFile, &args), "xyz\nabc\nxy\nxz\nyz\n", "for {MultipleByFile:?}");
    }

    // Test `LogType::Lines` and `LogType::Files' output
    type CountMap = IndexMap<String, u32>;
    fn counted(operation: OpName, count: LogType, operands: &V8) -> CountMap {
        let first = operands[0];
        let rest = operands[1..].iter().map(|o| Ok(*o));
        let mut answer = Vec::new();
        calculate(operation, count, first, rest, &mut answer).unwrap();

        let mut result = CountMap::new();
        for line in String::from_utf8(answer).unwrap().lines() {
            let line = line.trim_start();
            let v: Vec<_> = line.splitn(2, ' ').collect();
            let count: u32 = v[0].parse().unwrap();
            result.insert(v[1].to_string(), count);
        }
        result
    }
    fn lines(operands: &V8) -> CountMap {
        let mut result = CountMap::new();
        for &operand in operands {
            let operand = String::from_utf8(operand.to_vec()).unwrap();
            for line in operand.lines() {
                result.entry(line.to_string()).and_modify(|c| *c += 1).or_insert(1);
            }
        }
        result
    }
    fn files(operands: &V8) -> CountMap {
        let mut result = CountMap::new();
        for &operand in operands {
            let operand = String::from_utf8(operand.to_vec()).unwrap();
            let mut seen = CountMap::new();
            for line in operand.lines() {
                seen.insert(line.to_string(), 1);
            }
            for line in seen.into_keys() {
                result.entry(line).and_modify(|c| *c += 1).or_insert(1);
            }
        }
        result
    }
    #[test]
    fn check_line_count() {
        let args: Vec<&[u8]> = vec![
            b"xyz\nabc\nxy\nxz\nx\n",    // Strings containing "x" (and "abc")
            b"xyz\nabc\nxy\nyz\ny\ny\n", // Strings containing "y" (and "abc")
            b"xyz\nabc\nxz\nyz\nz\n",    // Strings containing "z" (and "abc")
        ];
        let line_count = lines(&args);
        for &op in &[Intersect, Union, Diff, Single, SingleByFile, Multiple, MultipleByFile] {
            let result = counted(op, LogType::Lines, &args);
            for line in result.keys() {
                assert_eq!(result.get(line), line_count.get(line));
            }
        }
    }
    #[test]
    fn check_file_count() {
        let args: Vec<&[u8]> = vec![
            b"xyz\nabc\nxy\nxz\nx\n",    // Strings containing "x" (and "abc")
            b"xyz\nabc\nxy\nyz\ny\ny\n", // Strings containing "y" (and "abc")
            b"xyz\nabc\nxz\nyz\nz\n",    // Strings containing "z" (and "abc")
        ];
        let file_count = files(&args);
        for &op in &[Intersect, Union, Diff, Single, SingleByFile, Multiple, MultipleByFile] {
            let result = counted(op, LogType::Files, &args);
            for line in result.keys() {
                assert_eq!(result.get(line), file_count.get(line));
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::pedantic)]
mod test_bookkeeping {
    use super::*;
    use std::fs::File;

    trait Testable: Copy + PartialEq + Debug {
        fn file_number(self) -> Option<u32> {
            None
        }
        fn set_file_number(&mut self, file_number: u32) {}
        fn set_line_count(&mut self, line_count: u32) {}
    }

    impl Testable for Noop {}
    impl Testable for LastFileSeen {
        fn file_number(self) -> Option<u32> {
            Some(self.0)
        }
        fn set_file_number(&mut self, file_number: u32) {
            self.0 = file_number
        }
    }
    impl Testable for LineCount {
        fn set_line_count(&mut self, line_count: u32) {
            self.0 = line_count;
        }
    }
    impl Testable for FileCount {
        fn file_number(self) -> Option<u32> {
            Some(self.file_number)
        }
        fn set_file_number(&mut self, file_number: u32) {
            self.file_number = file_number
        }
    }
    impl<R: Bookkeeping + Testable, B: Bookkeeping + Testable> Testable for Dual<R, B> {
        fn file_number(self) -> Option<u32> {
            self.retention.file_number().or(self.log.file_number())
        }
        fn set_file_number(&mut self, file_number: u32) {
            self.retention.set_file_number(file_number);
            self.log.set_file_number(file_number);
        }
        fn set_line_count(&mut self, line_count: u32) {
            self.log.set_line_count(line_count);
        }
    }
    impl<R: Bookkeeping + Testable> Testable for Logged<R> {
        fn file_number(self) -> Option<u32> {
            self.0.file_number()
        }
        fn set_file_number(&mut self, file_number: u32) {
            self.0.set_file_number(file_number)
        }
        fn set_line_count(&mut self, line_count: u32) {
            self.0.set_line_count(line_count);
        }
    }
    impl<R: Bookkeeping + Testable> Testable for Unlogged<R> {
        fn file_number(self) -> Option<u32> {
            self.0.file_number()
        }
        fn set_file_number(&mut self, file_number: u32) {
            self.0.set_file_number(file_number)
        }
        fn set_line_count(&mut self, line_count: u32) {
            self.0.set_line_count(line_count);
        }
    }
    fn new_file_number<R: Bookkeeping + Testable>() -> Option<u32> {
        R::new().file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn first_file_file_number_is_None_for_Noop_and_LineCount_and_Some_0_otherwise() {
        assert_eq!(new_file_number::<Dual<FileCount, LineCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<LastFileSeen, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<LastFileSeen, LineCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<LineCount, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<Noop, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<Noop, LineCount>>(), None);
        assert_eq!(new_file_number::<Logged<FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Logged<LineCount>>(), None);
        assert_eq!(new_file_number::<Unlogged<FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Unlogged<LastFileSeen>>(), Some(0));
        assert_eq!(new_file_number::<Unlogged<LineCount>>(), None);
        assert_eq!(new_file_number::<Unlogged<Noop>>(), None);
    }

    fn bump_twice<R: Bookkeeping>() -> R {
        let mut select = R::new();
        select.next_file().unwrap();
        select.next_file().unwrap();
        select
    }
    fn bump_twice_file_number<R: Bookkeeping + Testable>() -> Option<u32> {
        bump_twice::<R>().file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn next_file_increments_file_number_only_for_LastFileSeen_and_FileCount() {
        assert_eq!(bump_twice_file_number::<Dual<FileCount, LineCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, LineCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LineCount, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<Noop, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<Noop, LineCount>>(), None);
        assert_eq!(bump_twice_file_number::<Logged<FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Logged<LineCount>>(), None);
        assert_eq!(bump_twice_file_number::<Unlogged<FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Unlogged<LastFileSeen>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Unlogged<LineCount>>(), None);
        assert_eq!(bump_twice_file_number::<Unlogged<Noop>>(), None);
    }

    macro_rules! check {
        ($assert:ident) => {
            $assert::<Dual<FileCount, LineCount>>();
            $assert::<Dual<LastFileSeen, FileCount>>();
            $assert::<Dual<LastFileSeen, LineCount>>();
            $assert::<Dual<LineCount, FileCount>>();
            $assert::<Dual<Noop, FileCount>>();
            $assert::<Dual<Noop, LineCount>>();
            $assert::<Logged<FileCount>>();
            $assert::<Logged<LineCount>>();
            $assert::<Unlogged<FileCount>>();
            $assert::<Unlogged<LastFileSeen>>();
            $assert::<Unlogged<LineCount>>();
            $assert::<Unlogged<Noop>>();
        };
    }
    fn assert_update_with_sets_self_file_number_to_arguments<R: Bookkeeping + Testable>() {
        let mut naive = R::new();
        let mut other = R::new();
        other.next_file().unwrap();
        other.next_file().unwrap();
        naive.update_with(other);
        assert_eq!(naive.file_number(), other.file_number());
    }
    #[test]
    fn update_with_sets_file_number_to_its_arguments_file_number() {
        check!(assert_update_with_sets_self_file_number_to_arguments);
    }

    #[allow(non_snake_case)]
    fn assert_next_file_errors_if_file_number_is_u32_MAX<R: Bookkeeping + Testable>() {
        let mut item = R::new();
        let start = item.file_number();
        item.next_file().unwrap();
        if item.file_number() == start {
            return;
        }
        item.set_file_number(u32::MAX - 2);
        item.next_file().unwrap();
        assert!(item.file_number() == Some(u32::MAX - 1));
        item.next_file().unwrap();
        assert!(item.file_number() == Some(u32::MAX));
        assert!(item.next_file().is_err());
    }
    #[test]
    fn next_file_errors_if_file_number_would_wrap_to_zero() {
        check!(assert_next_file_errors_if_file_number_is_u32_MAX);
    }

    fn log_string<B: Bookkeeping + Testable>(item: B) -> String {
        let mut result = vec![];
        item.write_count(10, &mut result).unwrap();
        String::from_utf8(result).unwrap()
    }
    fn assert_item_logs_overflow_when_appropriate<B: Bookkeeping + Testable>() {
        let mut item = B::new();
        item.set_line_count(42);
        if log_string(item).trim() == "42" {
            // Otherwise we're not counting lines
            let big_but_ok = u32::MAX - 1;
            item.set_line_count(big_but_ok);
            assert_eq!(log_string(item).trim(), format!("{big_but_ok}"));

            // Simulate seeing another line
            item.update_with(item);
            assert_eq!(log_string(item).trim(), "overflow");

            // And yet another line – Once line count hits overflow, it doesn't change.
            item.update_with(item);
            assert_eq!(log_string(item).trim(), "overflow");
        }
    }
    #[test]
    fn item_logs_overflow_when_appropriate() {
        check!(assert_item_logs_overflow_when_appropriate);
    }
}

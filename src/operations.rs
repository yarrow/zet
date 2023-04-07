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
            Union => union::<Noop, O>(first_operand, rest, out),
            Diff => diff::<LastFileSeen, O>(first_operand, rest, out),
            Intersect => intersect::<LastFileSeen, O>(first_operand, rest, out),
            Single => count::<CountLines, O>(AndKeep::Single, first_operand, rest, out),
            Multiple => count::<CountLines, O>(AndKeep::Multiple, first_operand, rest, out),
            SingleByFile => count::<CountFiles, O>(AndKeep::Single, first_operand, rest, out),
            MultipleByFile => count::<CountFiles, O>(AndKeep::Multiple, first_operand, rest, out),
        },

        // When `log_type` is `LogType::Lines` and `operation` is `Single` or
        // `Multiple`, both logging and selection use `CountLines`. Since
        // `Dual<CountLines, CountLines>` would do duplicate bookkeeping, we just
        // use `CountLines` by itself.
        LogType::Lines => match operation {
            Union => union::<Dual<Noop, LogLines>, O>(first_operand, rest, out),
            Diff => diff::<Dual<LastFileSeen, LogLines>, O>(first_operand, rest, out),
            Intersect => intersect::<Dual<LastFileSeen, CountLines>, O>(first_operand, rest, out),
            Single => count::<LogLines, O>(AndKeep::Single, first_operand, rest, out),
            Multiple => count::<LogLines, O>(AndKeep::Multiple, first_operand, rest, out),
            SingleByFile => {
                count::<Dual<CountFiles, LogLines>, O>(AndKeep::Single, first_operand, rest, out)
            }
            MultipleByFile => {
                count::<Dual<CountFiles, LogLines>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
        },

        // Similarly, we don't want to use `Dual<CountFiles, CountFiles>`
        // bookkeeping values, so we use `LogFiles` by itselfwhen `log_type` is
        // LogType::Files` and `operation` is `SingleByFile` or
        // `MultipleByFile`.
        //
        // And we use `LogLines` for `Single`, rather than `Dual<CountLines,
        // CountFiles>`, since the number reported for `Single` will always be 1
        // — a line appearing only once can appear in only one file.
        LogType::Files => match operation {
            Union => union::<Dual<Noop, LogFiles>, O>(first_operand, rest, out),
            Diff => diff::<Dual<LastFileSeen, LogFiles>, O>(first_operand, rest, out),
            Intersect => intersect::<Dual<LastFileSeen, LogFiles>, O>(first_operand, rest, out),
            Single => count::<LogLines, O>(AndKeep::Single, first_operand, rest, out),
            Multiple => {
                count::<Dual<CountLines, LogFiles>, O>(AndKeep::Multiple, first_operand, rest, out)
            }
            SingleByFile => count::<LogFiles, O>(AndKeep::Single, first_operand, rest, out),
            MultipleByFile => count::<LogFiles, O>(AndKeep::Multiple, first_operand, rest, out),
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
    fn output_zet_set(set: &ZetSet<Self>, mut out: impl std::io::Write) -> Result<()> {
        out.write_all(set.bom)?;
        for line in set.keys() {
            out.write_all(line)?;
            out.write_all(set.line_terminator)?;
        }
        out.flush()?;
        Ok(())
    }
}

trait Loggable {
    fn log_value(self) -> u32;
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()>;
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Logged<R: Bookkeeping + Loggable>(R);
impl<R: Bookkeeping + Loggable> Bookkeeping for Logged<R> {
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
    fn output_zet_set(set: &ZetSet<Self>, mut out: impl std::io::Write) -> Result<()> {
        let Some(max_count) = set.values().map(|v| v.log_value()).max() else { return Ok(()) };
        let width = (max_count.ilog10() + 1) as usize;
        out.write_all(set.bom)?;
        for (line, item) in set.iter() {
            item.write_log(width, &mut out)?;
            out.write_all(line)?;
            out.write_all(set.line_terminator)?;
        }
        out.flush()?;
        Ok(())
    }
}
impl<R: Bookkeeping + Loggable> Loggable for Logged<R> {
    fn log_value(self) -> u32 {
        self.0.log_value()
    }
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        self.0.write_log(width, out)
    }
}

type Dual<B, C> = Logged<Duo<B, C>>;
/// The `Dual` struct lets us use one item for retention purposes and another
/// for logging. We take the `retention_value` from the first item and `log_value`
/// and `write_log` from the second.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Duo<Retain: Bookkeeping, Log: Bookkeeping + Loggable> {
    pub(crate) retention: Retain,
    pub(crate) log: Log,
}
impl<Retain: Bookkeeping, Log: Bookkeeping + Loggable> Bookkeeping for Duo<Retain, Log> {
    fn new() -> Self {
        Duo { retention: Retain::new(), log: Log::new() }
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
}
impl<Retain: Bookkeeping, Log: Bookkeeping + Loggable> Loggable for Duo<Retain, Log> {
    fn log_value(self) -> u32 {
        self.log.retention_value()
    }
    fn write_log(&self, width: usize, mut out: &mut impl std::io::Write) -> Result<()> {
        self.log.write_log(width, &mut out)
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

/// For `Single` and `Multiple` each line's `CountLines` item will keep track of
/// how many times it has appeared in the entire input. `CountLines` can also be
/// used for reporting the number of times each line appears in the input.
///
/// Like `LastFileSeen`, `CountLines` is a thin wrapper around `u32` — but
/// `CountLines` ignores `next_file`, and uses `update_with` only to increment the
/// `u32`. Here we use a saturating increment, because neither `Single` and
/// `Multiple` care only whether the `u32` is `1` or greater than `1`, and for
/// logging purposes it seems better to report overflow for lines that appear
/// `u32::MAX` times or more than to stop `zet` completely.
#[derive(Clone, Copy, PartialEq, Debug)]
struct CountLines(u32);
impl Bookkeeping for CountLines {
    fn new() -> Self {
        CountLines(1)
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
}
impl Loggable for CountLines {
    fn log_value(self) -> u32 {
        self.retention_value()
    }
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        if self.0 == u32::MAX {
            write!(out, " overflow  ")?
        } else {
            write!(out, "{:width$} ", self.0)?
        }
        Ok(())
    }
}
type LogLines = Logged<CountLines>;
type LogFiles = Logged<CountFiles>;
/// For `SingleByFile` and `MultipleByFile` each line's `CountFiles` item will
/// keep track of how many files the line has appeared in. `CountFiles` can also
/// be used to report the file count information for operatons whose selection
/// criteria are different from number of files.
///
/// Like `LastFileSeen`, `CountFiles` keeps track of the last file seen, and
/// `bail`s if the number of files seen exceeds `u32::MAX`. It has a separate
/// `files_seen` field for tracking the number of files seen.
#[derive(Clone, Copy, PartialEq, Debug)]
struct CountFiles {
    file_number: u32,
    files_seen: u32,
}
impl Bookkeeping for CountFiles {
    fn new() -> Self {
        CountFiles { file_number: 0, files_seen: 1 }
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
}
impl Loggable for CountFiles {
    fn log_value(self) -> u32 {
        self.retention_value()
    }
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
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
    B::output_zet_set(&set, out)?;
    std::mem::forget(set); // Slightly faster to just abandon this, since we're about to exit.
                           // Thanks to [Karolin Varner](https://github.com/koraa)'s huniq
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
            let result = counted(dbg!(op), LogType::Lines, &args);
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

    #[test]
    fn line_count_update_with_uses_saturating_increment() {
        let mut changer = CountLines(u32::MAX - 2);
        let other = CountLines::new();
        assert_eq!(changer.retention_value(), u32::MAX - 2);
        changer.update_with(other);
        assert_eq!(changer.retention_value(), u32::MAX - 1);
        changer.update_with(other);
        assert_eq!(changer.retention_value(), u32::MAX);
        changer.update_with(other);
        assert_eq!(changer.retention_value(), u32::MAX);
    }

    #[test]
    fn file_count_next_file_uses_checked_increment() {
        let mut changer = CountFiles { file_number: u32::MAX - 1, files_seen: 1 };
        changer.next_file().unwrap();
        assert_eq!(changer, CountFiles { file_number: u32::MAX, files_seen: 1 });
        assert!(changer.next_file().is_err());
    }

    #[test]
    fn last_file_seen_next_file_uses_checked_increment() {
        let mut changer = LastFileSeen(u32::MAX - 1);
        changer.next_file().unwrap();
        assert_eq!(changer, LastFileSeen(u32::MAX));
        assert!(changer.next_file().is_err());
    }

    #[test]
    fn log_lines_logs_the_string_overflow_for_u32_max() {
        let zet = ZetSet::<LogLines>::new(b"a\na\na\nb\n", Logged(CountLines(u32::MAX - 1)));
        let mut result = Vec::new();
        LogLines::output_zet_set(&zet, &mut result).unwrap();
        let result = String::from_utf8(result).unwrap();
        assert_eq!(result, format!(" overflow  a\n{} b\n", u32::MAX - 1));
    }
}

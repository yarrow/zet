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
/// of times each line appears in the input (`LogType::Lines`), the number of
/// files in which each line appears (`LogType::Files`), or neither
/// (`LogType::None`).
///
pub fn calculate<O: LaterOperand>(
    operation: OpName,
    log_type: LogType,
    first_operand: &[u8],
    rest: impl ExactSizeIterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let number_of_operands = rest.len() + 1; // + 1 because first_operand is an operand
    if number_of_operands > u32::MAX as usize {
        bail!("Zet can't handle more than {} input files", u32::MAX)
        // Since we have <= u32::MAX operands, the `next_file` method can't overflow and we can use
        // wrapping_add
    }
    match log_type {
        LogType::None => match operation {
            Union => union::<Unsifted, O>(first_operand, rest, out),
            Diff => diff::<Files, O>(first_operand, rest, out),
            Intersect => intersect::<Files, O>(first_operand, rest, out),
            Single => keep_single::<Lines, O>(first_operand, rest, out),
            Multiple => keep_multiple::<Lines, O>(first_operand, rest, out),
            SingleByFile => keep_single::<Files, O>(first_operand, rest, out),
            MultipleByFile => keep_multiple::<Files, O>(first_operand, rest, out),
        },

        // When `log_type` is `LogType::Lines` and `operation` is `Single` or
        // `Multiple`, both logging and selection use `Lines`. Since
        // `SiftLog<Lines, Lines>` would do duplicate bookkeeping, we just
        // use `Lines` by itself.
        LogType::Lines => match operation {
            Union => union::<Log<Lines>, O>(first_operand, rest, out),
            Diff => diff::<SiftLog<Files, Lines>, O>(first_operand, rest, out),
            Intersect => intersect::<SiftLog<Files, Lines>, O>(first_operand, rest, out),
            Single => keep_single::<Log<Lines>, O>(first_operand, rest, out),
            Multiple => keep_multiple::<Log<Lines>, O>(first_operand, rest, out),
            SingleByFile => keep_single::<SiftLog<Files, Lines>, O>(first_operand, rest, out),
            MultipleByFile => keep_multiple::<SiftLog<Files, Lines>, O>(first_operand, rest, out),
        },

        // Similarly, we don't want to use `SiftLog<Files, Files>` bookkeeping
        // values, so we use `Log<Files>` by itself when `log_type` is
        // LogType::Files` and `operation` is `SingleByFile` or
        // `MultipleByFile`.
        //
        // And we use `Log<Lines>` for `Single`, rather than `SiftLog<Lines,
        // Files>`, since the number reported for `Single` will always be 1 — a
        // line appearing only once can appear in only one file.
        LogType::Files => match operation {
            Union => union::<Log<Files>, O>(first_operand, rest, out),
            Diff => diff::<Log<Files>, O>(first_operand, rest, out),
            Intersect => intersect::<Log<Files>, O>(first_operand, rest, out),
            Single => keep_single::<Log<Lines>, O>(first_operand, rest, out),
            Multiple => keep_multiple::<SiftLog<Lines, Files>, O>(first_operand, rest, out),
            SingleByFile => keep_single::<Log<Files>, O>(first_operand, rest, out),
            MultipleByFile => keep_multiple::<Log<Files>, O>(first_operand, rest, out),
        },
    }
}

/// A `ZetSet` is an ordered set of lines where each line from the input file(s)
/// occurs once in the `ZetSet`, and each line has an associated `Bookkeeping`
/// value that we use to determine whether to retain the line in the output, and
/// optionally to output a count along with each line (counting either the
/// number of times the line occurs in the input, or the number of files in
/// which the line occurs).
///
/// The `Bookkeeping` trait specifies the kind of types that can serve as the
/// bookkeeping values for a `ZetSet`, and defines a default `output_zet_set`
/// method to print the lines without a count.
///
/// There are seven `Bookkeeping` types. The `Unsifted`, `Lines`, and `Files`
/// types are used for "sifting" — after all files have been processed, we look
/// at the bookkeeping values to sift out unwanted lines before printing.  The
/// `Union` operation outputs every line, so uses an `Unsifted` bookkeeping type
/// with a zero-size value and no-op methods.  The `Single` and `Multiple`
/// operations use the `Lines` type to sift by the number of times a line has
/// been seen, while the `Diff`, `Intersect`, `SingleByFile`, and
/// `MultipleByFile` operations use the `Files` type to sift by the number of
/// files in which a line has been seen.
///
/// The `Log<Lines>` and `Log<Files>` types act like `Lines` and `Files`
/// respectively, except that their `output_zet_set` methods output the
/// appropriate count along with each line. They can also be used for sifting,
/// so if we want to output only those lines which occur more than once in the
/// input, and want to know how many times each line has been seen, we can use
/// `Log<Lines>` both retain lines seen more than once and to print the exact
/// number.
///
/// Sometimes, though we want to sift by one value but print another. We might,
/// for instance, want to output lines that occur in only one file, but also
/// want to print how many time each line occurred in the file. For that we'd
/// use `SiftLog<Files, Lines>` bookkeeping values to sift by the number of
/// files seen and log the number of lines seen.  And we could use
/// `SiftLog<Lines, Files>` to print only lines occuring multiple times, while
/// printing the number of files each line occurs in.
pub(crate) trait Bookkeeping: Copy + PartialEq + Debug {
    /// The initial bookkeeping value for each line in the first operand.
    /// Usually keeps track of lines and/or files seen.
    fn new() -> Self;

    /// Increment the bookkeeping item's `n`th file field (if it has one)
    fn next_file(&mut self);

    /// Here `other` is the value that would have been inserted for a
    /// newly-encountered line. Used to update the bookkeeping values of lines
    /// already present in the `ZetSet`.
    fn update_with(&mut self, other: Self);

    /// The value to be used in closure passed to the `ZetSet`'s `retain`
    /// method.
    fn retention_value(self) -> u32;

    /// Output the `ZetSet`. The provided implementation doesn't log a count of
    /// lines or files, so must be overridden by types that do loggging.
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

/// The `Loggable` trait specifies two additional methods used to log a count
/// with each output line.
trait Loggable: Bookkeeping {
    /// The line/file count to be used for logging purposes
    fn log_value(self) -> u32;

    /// Write the count to the output. Called before outputting the line itself.
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()>;
}

/// For the "additive" operations (all but `Diff` and `Intersect`), we insert
/// every line in the input into the `ZetSet`. Both `ZetSet::new` and
/// `set.insert_or_update` will call `b.update_with(item)` on the line's
/// bookkeeping item `b` if the line is already present in the `ZetSet`.
///
/// `every_line`'s caller can then use `set.retain()` to examine the each line's
/// bookkeeping item to decide whether or not it belongs in the set.
fn every_line<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
) -> Result<ZetSet<B>> {
    let mut item = B::new();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file();
        set.insert_or_update(operand?, item)?;
    }
    Ok(set)
}

/// `Union` collects every line, so we don't need to call `retain`
fn union<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let set = every_line::<B, O>(first_operand, rest)?;
    output_and_discard(set, out)
}

/// `Single` and `SingleByFile` retain those lines where the relevant count is
/// `1`.
fn keep_single<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut set = every_line::<B, O>(first_operand, rest)?;
    set.retain(|occurences| occurences == 1);
    output_and_discard(set, out)
}

/// `Multiple` and `MultipleByFile` retain those lines where the relevant count is
/// greater than `1`.
fn keep_multiple<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let mut set = every_line::<B, O>(first_operand, rest)?;
    set.retain(|occurences| occurences > 1);
    output_and_discard(set, out)
}

/// For the "subtractive" operations `Diff` and `Intersect`, we insert only
/// those lines in the first input file into the `ZetSet`. `ZetSet::new` will
/// call `b.update_with(item)` on the line's bookkeeping item `b` if the line is
/// already present in the `ZetSet`.
///
/// Lines in the remaining files are only used to reduce the output, so we call
/// `set.update_if_present` to call `b.update_with(item)` when the line is
/// present in the `ZetSet` will bookkeeping value `b`.
///
/// Then the caller of `first_file_lines` can then use `set.retain()` to examine
/// the each line's bookkeeping item to decide whether or not it belongs in the
/// set.
fn first_file_lines<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
) -> Result<ZetSet<B>> {
    let mut item = B::new();
    let mut set = ZetSet::new(first_operand, item);
    for operand in rest {
        item.next_file();
        set.update_if_present(operand?, item)?;
    }
    Ok(set)
}

/// `Diff` retains only those lines seen only in the first file. Since
/// `first_file_lines` only includes lines from the first file, we can
/// equivalently retain those lines whose file count is `1`.
fn diff<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl Iterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let first_file_only = 1;
    let mut set = first_file_lines::<B, O>(first_operand, rest)?;
    set.retain(|files_containing_line| files_containing_line == first_file_only);
    output_and_discard(set, out)
}

/// `Intersect` retains only those lines whose file count is the same as the
/// number of input files.
fn intersect<B: Bookkeeping, O: LaterOperand>(
    first_operand: &[u8],
    rest: impl ExactSizeIterator<Item = Result<O>>,
    out: impl std::io::Write,
) -> Result<()> {
    let all_files = u32::try_from(rest.len() + 1)?;
    let mut set = first_file_lines::<B, O>(first_operand, rest)?;
    set.retain(|files_containing_line| files_containing_line == all_files);
    output_and_discard(set, out)
}

/// When we've finished constructing the `ZetSet`, we write its lines to our
/// output and exit the program.
fn output_and_discard<B: Bookkeeping>(set: ZetSet<B>, out: impl std::io::Write) -> Result<()> {
    B::output_zet_set(&set, out)?;
    std::mem::forget(set); // Slightly faster to just abandon this, since we're about to exit.
                           // Thanks to [Karolin Varner](https://github.com/koraa)'s huniq
    Ok(())
}

/// We use the `Unsifted` struct for the `Union` operation when logging isn't needed.
/// `Union` includes every line seen and doesn't need bookkeeping for anything
/// but such logging.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Unsifted();
impl Bookkeeping for Unsifted {
    fn new() -> Self {
        Unsifted()
    }
    fn next_file(&mut self) {}
    fn update_with(&mut self, _other: Self) {}
    fn retention_value(self) -> u32 {
        0
    }
}

/// For `Single` and `Multiple` each line's `Lines` item will keep track of
/// how many times it has appeared in the entire input. `Lines` can also be
/// used for reporting the number of times each line appears in the input.
///
/// `Lines` is a thin wrapper around `u32`. It ignores `next_file`, and uses
/// `update_with` only to increment its `u32` element. We use a saturating
/// increment, because neither `Single` and `Multiple` care only whether the
/// `u32` is `1` or greater than `1`, and for logging purposes it seems better
/// to report overflow for lines that appear `u32::MAX` times or more than to
/// stop `zet` completely.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Lines(u32);
impl Bookkeeping for Lines {
    /// Returns `Lines(1)` because when we insert a fresh line into the `ZetSet`
    /// we've seen it once.
    fn new() -> Self {
        Lines(1)
    }

    /// `next_file` does nothing because `Lines` isn't affected by the number of
    /// files we've seen.
    fn next_file(&mut self) {}

    /// When `update_with` is called, it means we've seen the line an additional
    /// time.  We ignore `_other` and just increment our line count (with
    /// `saturating_add(1)` so we don't wrap around.
    fn update_with(&mut self, _other: Self) {
        self.0 = self.0.saturating_add(1);
    }

    /// Our `retention_value` is just the `u32` element.
    fn retention_value(self) -> u32 {
        self.0
    }
}
impl Loggable for Lines {
    /// Our `log_value` is the same as our `retention_value`: the underlying
    /// `u32` element.
    fn log_value(self) -> u32 {
        self.retention_value()
    }

    /// Write our `log_value`. But if that is `u32::MAX`, write `" overflow  "`
    /// instead, since we might actually have seen more than `u32::MAX` lines.
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        if self.0 == u32::MAX {
            write!(out, " overflow  ")?
        } else {
            write!(out, "{:width$} ", self.0)?
        }
        Ok(())
    }
}
/// For `Diff`, `Intersect`, `SingleByFile`, and `MultipleByFile`, each line's
/// `Files` item will keep track of how many files the line has appeared in.
/// `Files` can also be used to report the file count information for operatons
/// whose selection criteria are different from number of files.
///
/// The `Files` struct has `file_number` and `files_seen` fields.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Files {
    file_number: u32,
    files_seen: u32,
}
impl Bookkeeping for Files {
    /// Returns `Files { file_number: 0, files_seen: 1 }` — `file_number` acts
    /// as an ID number, different for each operand, while `files_seen` counts
    /// the number of files this line has been seen to occur in.
    fn new() -> Self {
        Files { file_number: 0, files_seen: 1 }
    }

    /// Increment the `file_number` field — with `wrapping_add(1)` because we
    /// trust `calculate` to have bailed if there are more than `u32::MAX` file
    /// operands.
    fn next_file(&mut self) {
        self.file_number = self.file_number.wrapping_add(1);
    }

    /// If a line is already present in the `ZetSet`, with bookkeeping value
    /// `b`, and `other.file_number` is different from `b.file_number`, we
    /// update `b.file_number` and increment `b.files_seen`.
    fn update_with(&mut self, other: Self) {
        if other.file_number != self.file_number {
            self.files_seen += 1;
            self.file_number = other.file_number;
        }
    }

    /// Our `retention_value` is the `files_seen` field.
    fn retention_value(self) -> u32 {
        self.files_seen
    }
}
impl Loggable for Files {
    /// Our `log_value` is the same as our `retention_value` — `files_seen`.
    fn log_value(self) -> u32 {
        self.retention_value()
    }

    /// We write `files_seen`.
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        write!(out, "{:width$} ", self.files_seen)?;
        Ok(())
    }
}

/// The `Log` newtype delegates everything except `output_zet_set` to its
/// sole element, and overrides `output_zet_set` to call
/// `output_zet_set_annotated`.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Log<B: Loggable>(B);
impl<B: Loggable> Bookkeeping for Log<B> {
    fn new() -> Self {
        Self(B::new())
    }
    fn next_file(&mut self) {
        self.0.next_file()
    }
    fn update_with(&mut self, other: Self) {
        self.0.update_with(other.0)
    }
    fn retention_value(self) -> u32 {
        self.0.retention_value()
    }
    fn output_zet_set(set: &ZetSet<Self>, out: impl std::io::Write) -> Result<()> {
        output_zet_set_annotated(set, out)
    }
}
impl<B: Loggable> Loggable for Log<B> {
    fn log_value(self) -> u32 {
        self.0.log_value()
    }
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        self.0.write_log(width, out)
    }
}

/// The two `Loggable` methods are used in `output_zet_set_annotated`, and the
/// `Log<X>` and `SiftLog<X,Y>` types override `output_zet_set` to call
/// `output_zet_set_annotated` for the actual logging.
fn output_zet_set_annotated<B: Loggable>(
    set: &ZetSet<B>,
    mut out: impl std::io::Write,
) -> Result<()> {
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

/// A `SiftLog<Sifted, Logged>` struct tracks a `Bookkeeping` item of type
/// `Sifted` and a `Loggable` item of type `Logged`. The latter will be used to
/// print a count for each line, either the number of times the line appeared in
/// the input, or the number of files it appeared in. We use the
/// `retention_value` of `Sifted` and the `log_value` and `write_log` methods of
/// `Logged`.
#[derive(Clone, Copy, PartialEq, Debug)]
struct SiftLog<Sifted: Bookkeeping, Logged: Loggable> {
    sift: Sifted,
    log: Logged,
}
impl<Sifted: Bookkeeping, Logged: Loggable> Bookkeeping for SiftLog<Sifted, Logged> {
    /// Returns `SiftLog { sift: Sifted::new(), log: Logged::new() }` —
    /// freshly inserted lines will have a bookkeeping item suitable for both
    /// sifting and logging.
    fn new() -> Self {
        SiftLog { sift: Sifted::new(), log: Logged::new() }
    }

    /// Our `next_file` method calls `next_file` for both its fields.
    fn next_file(&mut self) {
        self.sift.next_file();
        self.log.next_file()
    }

    /// Our `update_with` method calls `update_with` for both its fields,
    /// sending `other.sift` to our `sift` field and `other.log` to our `log`
    /// field.
    fn update_with(&mut self, other: Self) {
        self.sift.update_with(other.sift);
        self.log.update_with(other.log);
    }

    /// Our `retention_value` is our **`sift` field's** retention value.
    fn retention_value(self) -> u32 {
        self.sift.retention_value()
    }

    /// We override `output_zet_set` to use `output_zet_set_annotated`.
    fn output_zet_set(set: &ZetSet<Self>, out: impl std::io::Write) -> Result<()> {
        output_zet_set_annotated(set, out)
    }
}
impl<Sifted: Bookkeeping, Logged: Loggable> Loggable for SiftLog<Sifted, Logged> {
    /// Our `log_value` is our **`log` field's** log value.
    fn log_value(self) -> u32 {
        self.log.log_value()
    }

    /// For `write_log` we output our `log` field's log value.
    fn write_log(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        self.log.write_log(width, out)
    }
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

    #[test]
    fn line_count_update_with_uses_saturating_increment() {
        let mut changer = Lines(u32::MAX - 2);
        let other = Lines::new();
        assert_eq!(changer.retention_value(), u32::MAX - 2);
        changer.update_with(other);
        assert_eq!(changer.retention_value(), u32::MAX - 1);
        changer.update_with(other);
        assert_eq!(changer.retention_value(), u32::MAX);
        changer.update_with(other);
        assert_eq!(changer.retention_value(), u32::MAX);
    }

    #[test]
    fn log_lines_logs_the_string_overflow_for_u32_max() {
        let zet = ZetSet::<Log<Lines>>::new(b"a\na\na\nb\n", Log(Lines(u32::MAX - 1)));
        let mut result = Vec::new();
        Log::<Lines>::output_zet_set(&zet, &mut result).unwrap();
        let result = String::from_utf8(result).unwrap();
        assert_eq!(result, format!(" overflow  a\n{} b\n", u32::MAX - 1));
    }
}

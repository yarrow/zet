//! Provides the `ZetSet` structure, intended to be initialized from the
//! contents of the first input file.
use crate::tally::Log;
use anyhow::Result;
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use memchr::memchr;
use std::borrow::Cow;
use std::io;

/// A `ZetSet` is a set of lines, each line represented as a key of an `IndexMap`.
/// * Keys are `Cow<'data, [u8]>`
/// * Lines inserted from the first file operand are represented as `Cow::Borrowed` keys
/// * Lines inserted from the second and following files are represented as `Cow::Owned` keys
/// * Each set operation (`Union`, `Diff`, etc) associates a small bookkeeping value
///   with each key. The value type differs from operation to operation, and by whether we're
///   counting the number of times each line appears, or the number of files in which each
///   lines appears (or if we're not counting either).
/// * A `ZetSet` also keeps information about whether the first file operand began with
///   a Unicode Byte Order Mark, and what line terminator was used on the first line of
///   the first file. On output, the `ZetSet` will print a Byte Order Mark if the first
///   file operand had one, and will use the same line terminator as that file's first
///   line.
#[derive(Clone, Debug)]
pub(crate) struct ZetSet<'data, Bookkeeping: Log> {
    set: CowSet<'data, Bookkeeping>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

/// We don't, in fact, require the second and following "files" to be files! Our
/// only requirement is that they implement `for_byte_line`. The `LaterOperand`
/// trait codifies that.
pub trait LaterOperand {
    /// The call `o.for_byte_line(|line| ...)` method calls the given closure
    /// for each &[u8] in `o`.
    fn for_byte_line(self, for_each_line: impl FnMut(&[u8])) -> Result<()>;
}

/// When a `ZetSet` processes a line from an operand, it does one of two things:
/// * If the line is not present in the set, it is inserted, with a bookkeeping
/// value passed by the caller.
/// * If the line is already present in the set, `.modify(file_number)` is
/// called on its bookkeeping value.
///
/// The `new` function inserts lines borrowed from its `slice` argument. The
/// `insert_or_modify` inserts `Cow::Owned` lines, so its `operand` argument
/// need not outlive the `ZetSet` The `modify_if_present` method only modifies —
/// it's used by the `Insert` and `Diff` operations, which only decrease the set
/// returned by `new` and never add to it.
///
/// The `retain` method filters the set, using a function passed by the caller that
/// looks at the `.value()` of the bookkeeping item.
///
/// The `output_to` method prints the lines of the set, calling the bookkeeping
/// item's `write_count` method (when appropriate) to prefix each line with the
/// number of times it appears in the input, or the number of files it appears
/// in.
impl<'data, Bookkeeping: Log> ZetSet<'data, Bookkeeping> {
    /// Create a new `ZetSet`, with each key a line borrowed from `slice`, and
    /// value `Bookkeeping::new(1)` for every line — the correct `Bookkeeping`
    /// value for a line in the first file.
    ///
    /// Even though we know that we'll be inserting `Bookkeeping::new(1)`, we
    /// make the caller pass it in. Why make the caller pass a fixed value?  We
    /// need `item` not for its value, but its type — monomorphism needs to know
    /// the type of bookkeeping value we're using. So the choices are to make
    /// the caller pass in a value that we'll ignore, or to make the caller pass
    /// in the right value. The latter seems least bad.
    pub(crate) fn new(mut slice: &'data [u8], item: Bookkeeping) -> Self {
        assert!(item == Bookkeeping::new(1));
        let (bom, line_terminator) = output_info(slice);
        slice = &slice[bom.len()..];
        let mut set = CowSet::<Bookkeeping>::default();
        while let Some(end) = memchr(b'\n', slice) {
            let (mut line, rest) = slice.split_at(end);
            slice = &rest[1..];
            if let Some(&maybe_cr) = line.last() {
                if maybe_cr == b'\r' {
                    line = &line[..line.len() - 1];
                }
            }
            set.entry(Cow::Borrowed(line)).and_modify(|v| v.modify(1)).or_insert(item);
        }
        if !slice.is_empty() {
            set.entry(Cow::Borrowed(slice)).and_modify(|v| v.modify(1)).or_insert(item);
        }
        ZetSet { set, bom, line_terminator }
    }

    /// For each line in `operand`, insert `line` as `Cow::Owned` to the
    /// underlying `IndexMap` if it is not already present, with bookkeeping
    /// value `item`. If the line is present, call `modify` on the bookkeeping
    /// value.
    pub(crate) fn insert_or_modify(
        &mut self,
        operand: impl LaterOperand,
        file_number: u32,
        item: Bookkeeping,
    ) -> Result<()> {
        operand.for_byte_line(|line| {
            self.set
                .entry(Cow::from(line.to_vec()))
                .and_modify(|v| v.modify(file_number))
                .or_insert(item);
        })
    }

    /// For each line in `operand` that is already present in the underlying
    /// `IndexMap`, call `modify` on the bookkeeping value.
    pub(crate) fn modify_if_present(
        &mut self,
        operand: impl LaterOperand,
        file_number: u32,
    ) -> Result<()> {
        operand.for_byte_line(|line| {
            if let Some(bookkeeping) = self.set.get_mut(line) {
                bookkeeping.modify(file_number)
            }
        })
    }

    /// Like `IndexMap`'s `.retain` method, but exposes just the bookkeeping
    /// item's `.value()`
    pub(crate) fn retain(&mut self, keep: impl Fn(u32) -> bool) {
        self.set.retain(|_k, v| keep(v.value()));
    }

    /// Output the `ZetSet`'s lines with the appropriate Byte Order Mark and line
    /// terminator.
    pub(crate) fn output_to(&self, mut out: impl io::Write) -> Result<()> {
        let Some((_key, first)) = self.set.first() else { return Ok(()) };

        if first.count() == 0 {
            // We're counting neither lines nor files
            out.write_all(self.bom)?;
            for line in self.set.keys() {
                out.write_all(line)?;
                out.write_all(self.line_terminator)?;
            }
            out.flush()?;
        } else {
            // We're counting something
            let Some(max_count) = self.set.values().map(|v| v.count()).max() else { return Ok(()) };
            let width = (max_count.ilog10() + 1) as usize;
            out.write_all(self.bom)?;
            for (line, item) in self.set.iter() {
                item.write_count(width, &mut out)?;
                out.write_all(line)?;
                out.write_all(self.line_terminator)?;
            }
            out.flush()?;
        };

        Ok(())
    }
}

/// Returns `(bom, line_terminator)`, where `bom` is the (UTF-8) Byte Order
/// Mark, or the empty string if `slice` has none, and `line_terminator` is
/// `\r\n` if the first line of `slice` ends with `\r\n`, and `\n` if the first
/// line ends just with `\n` (or is the only line in the file and has no line
/// terminator).
fn output_info(slice: &[u8]) -> (&'static [u8], &'static [u8]) {
    let mut bom: &'static [u8] = b"";
    let mut line_terminator: &'static [u8] = b"\n";
    if has_bom(slice) {
        bom = BOM_BYTES;
    }
    if let Some(n) = memchr(b'\n', slice) {
        if n > 0 && slice[n - 1] == b'\r' {
            line_terminator = b"\r\n";
        }
    }
    (bom, line_terminator)
}

const BOM_0: u8 = b'\xEF';
const BOM_1: u8 = b'\xBB';
const BOM_2: u8 = b'\xBF';
const BOM_BYTES: &[u8] = b"\xEF\xBB\xBF";
/// Does `first_operand` begin with a (UTF-8) Byte Order Mark?
fn has_bom(first_operand: &[u8]) -> bool {
    first_operand.len() >= 3
        && first_operand[0] == BOM_0
        && first_operand[1] == BOM_1
        && first_operand[2] == BOM_2
}

#[allow(clippy::pedantic)]
#[cfg(test)]
mod test {
    use super::*;

    const UTF8_BOM: &str = "\u{FEFF}";

    #[test]
    fn utf8_bom_is_correct() {
        assert_eq!([BOM_0, BOM_1, BOM_2], UTF8_BOM.as_bytes());
    }
}

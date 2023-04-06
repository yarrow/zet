//! Provides the `ZetSet` structure, intended to be initialized from the
//! contents of the first input file.
use crate::operations::Bookkeeping;
use anyhow::Result;
use fxhash::FxBuildHasher;
use indexmap::{map, IndexMap};
use memchr::memchr;
use std::borrow::Cow;

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
pub(crate) struct ZetSet<'data, B: Bookkeeping> {
    set: CowSet<'data, B>,
    pub(crate) bom: &'static [u8], // Byte Order Mark or empty
    pub(crate) line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, B> = IndexMap<Cow<'data, [u8]>, B, FxBuildHasher>;

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
///   value `item` passed by the caller.
/// * If the line is already present in the set, `v.update_with(item)` is
///   called on its bookkeeping value `v`.
///
/// The `new` function inserts lines borrowed from its `slice` argument. The
/// `insert_or_update` inserts `Cow::Owned` lines, so its `operand` argument
/// need not outlive the `ZetSet` The `update_if_present` method only updates —
/// it's used by the `Insert` and `Diff` operations, which only decrease the set
/// returned by `new` and never add to it.
///
/// The `retain` method filters the set, using a function passed by the caller that
/// looks at the `.retention_value()` of the bookkeeping item.
///
/// The `output_to` method prints the lines of the set, calling the bookkeeping
/// item's `write_count` method (when appropriate) to prefix each line with the
/// number of times it appears in the input, or the number of files it appears
/// in.
impl<'data, B: Bookkeeping> ZetSet<'data, B> {
    /// Create a new `ZetSet`, with each key a line borrowed from `slice`, and
    /// value `item` for every line newly seen. If a line is already present,
    /// with bookkeeping value `v`, update it by calling `v.update_with(item)`
    pub(crate) fn new(mut slice: &'data [u8], item: B) -> Self {
        let (bom, line_terminator) = output_info(slice);
        slice = &slice[bom.len()..];
        let mut set = CowSet::<B>::default();
        while let Some(end) = memchr(b'\n', slice) {
            let (mut line, rest) = slice.split_at(end);
            slice = &rest[1..];
            if let Some(&maybe_cr) = line.last() {
                if maybe_cr == b'\r' {
                    line = &line[..line.len() - 1];
                }
            }
            set.entry(Cow::Borrowed(line)).and_modify(|v| v.update_with(item)).or_insert(item);
        }
        if !slice.is_empty() {
            set.entry(Cow::Borrowed(slice)).and_modify(|v| v.update_with(item)).or_insert(item);
        }
        ZetSet { set, bom, line_terminator }
    }

    /// For each line in `operand`, insert `line` as `Cow::Owned` to the
    /// underlying `IndexMap` if it is not already present, with bookkeeping
    /// value `item`. If `line` is already present, with bookkeeping value `v`,
    /// update it by calling `v.update_with(item)`
    pub(crate) fn insert_or_update(&mut self, operand: impl LaterOperand, item: B) -> Result<()> {
        operand.for_byte_line(|line| {
            self.set
                .entry(Cow::from(line.to_vec()))
                .and_modify(|v| v.update_with(item))
                .or_insert(item);
        })
    }

    /// For each line in `operand` that is already present in the underlying
    /// `IndexMap` with bookkeeping value `v`, call `v.update_with(item)`.
    pub(crate) fn update_if_present(&mut self, operand: impl LaterOperand, item: B) -> Result<()> {
        operand.for_byte_line(|line| {
            if let Some(bookkeeping) = self.set.get_mut(line) {
                bookkeeping.update_with(item)
            }
        })
    }

    /// Like `IndexMap`'s `.retain` method, but exposes just the bookkeeping
    /// item's `.retention_value()`
    pub(crate) fn retain(&mut self, keep: impl Fn(u32) -> bool) {
        self.set.retain(|_k, v| keep(v.retention_value()));
    }

    /// Expose the underlying `ZetSet`'s `keys` method
    pub(crate) fn keys(&self) -> map::Keys<Cow<[u8]>, B> {
        self.set.keys()
    }
    /// Expose the underlying `ZetSet`'s `iter` method
    pub(crate) fn iter(&self) -> map::Iter<Cow<[u8]>, B> {
        self.set.iter()
    }
    /// Expose the underlying `ZetSet`'s `values` method
    pub(crate) fn values(&self) -> map::Values<Cow<[u8]>, B> {
        self.set.values()
    }
    /// Expose the underlying `ZetSet`'s `first` method
    pub(crate) fn first(&self) -> Option<B> {
        self.set.first().map(|(_key, &first)| first)
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

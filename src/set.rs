//! Provides the `ZetSet` structure, intended to be initialized from the
//! contents of the first input file.
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
///   with each key. The value type differs from operation to operation.
/// * A `ZetSet` also keeps information about whether the first file operand began with
///   a Unicode Byte Order Mark, and what line terminator was used on the first line of
///   the first file. On output, the `ZetSet` will print a Byte Order Mark if the first
///   file operand had one, and will use the same line terminator as that file's first
///   line.
pub(crate) struct ZetSet<'data, B: Bookkeeping> {
    set: CowSet<'data, B>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

pub(crate) trait Bookkeeping: Copy {
    type Item: Copy;
    fn item(&self) -> Self::Item;
    fn get_mut_item(&mut self) -> &mut Self::Item;

    fn with_unit_line_count(item: Self::Item) -> Self;
    fn line_count(&self) -> u32;
    fn increment_line_count(&mut self);
}
pub(crate) fn zet_set_from<B: Bookkeeping>(mut slice: &[u8], info: B) -> ZetSet<B> {
    let (bom, line_terminator) = output_info(slice);
    slice = &slice[bom.len()..];
    let mut zet = ZetSet { set: CowSet::default(), bom, line_terminator };
    zet.insert_borrowed_lines(slice, info.item());
    zet
}
impl<'data, B: Bookkeeping> ZetSet<'data, B> {
    fn insert_borrowed(&mut self, line: &'data [u8], item: B::Item) {
        self.set
            .entry(Cow::Borrowed(line))
            .and_modify(B::increment_line_count)
            .or_insert_with(|| B::with_unit_line_count(item));
    }
    fn insert_borrowed_lines(&mut self, mut slice: &'data [u8], item: B::Item) {
        while let Some(end) = memchr(b'\n', slice) {
            let (mut line, rest) = slice.split_at(end);
            slice = &rest[1..];
            if let Some(&maybe_cr) = line.last() {
                if maybe_cr == b'\r' {
                    line = &line[..line.len() - 1];
                }
            }
            self.insert_borrowed(line, item);
        }
        if !slice.is_empty() {
            self.insert_borrowed(slice, item);
        }
    }
    /// Insert `line` as `Cow::Owned` to the underlying `IndexMap`
    pub(crate) fn insert(&mut self, line: &[u8], item: B::Item) {
        self.set
            .entry(Cow::from(line.to_vec()))
            .and_modify(B::increment_line_count)
            .or_insert_with(|| B::with_unit_line_count(item));
    }

    /// Sometimes we need to update the bookkeeping information
    pub(crate) fn get_mut(&mut self, line: &[u8]) -> Option<&mut B::Item> {
        self.set.get_mut(line).map(Bookkeeping::get_mut_item)
    }

    /// Like `IndexMap`'s `.retain` method, but exposes just the item, and by value.
    pub(crate) fn retain(&mut self, keep: impl Fn(B::Item) -> bool) {
        self.set.retain(|_k, v| keep(v.item()));
    }

    /// Retain lines seen just once
    pub(crate) fn retain_single(&mut self) {
        self.set.retain(|_k, v| v.line_count() == 1);
    }

    /// Retain lines seen more than once
    pub(crate) fn retain_multiple(&mut self) {
        self.set.retain(|_k, v| v.line_count() > 1);
    }

    /// Output the `ZetSet`'s lines with the appropriate Byte Order Mark and line
    /// terminator.
    pub(crate) fn output_to(&self, mut out: impl io::Write) -> Result<()> {
        out.write_all(self.bom)?;
        for line in self.set.keys() {
            out.write_all(line)?;
            out.write_all(self.line_terminator)?;
        }
        out.flush()?;
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

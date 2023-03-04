//! Provides the `UncountedSet` structure, intended to be initialized from the
//! contents of the first input file.
use anyhow::Result;
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use memchr::memchr;
use std::borrow::Cow;
use std::{io, ops::FnMut};

/// A `UncountedSet` is a set of lines, each line represented as a key of an `IndexMap`.
/// * Keys are `Cow<'data, [u8]>`
/// * Lines inserted from the first file operand are represented as `Cow::Borrowed` keys
/// * Lines inserted from the second and following files are represented as `Cow::Owned` keys
/// * Each set operation (`Union`, `Diff`, etc) associates a small bookkeeping value
///   with each key. The value type differs from operation to operation.
/// * A `UncountedSet` also keeps information about whether the first file operand began with
///   a Unicode Byte Order Mark, and what line terminator was used on the first line of
///   the first file. On output, the `UncountedSet` will print a Byte Order Mark if the first
///   file operand had one, and will use the same line terminator as that file's first
///   line.
pub(crate) struct UncountedSet<'data, Bookkeeping: Copy> {
    set: CowSet<'data, Bookkeeping>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

pub(crate) trait ZetSet<'data, Bookkeeping: Copy> {
    /// Insert `line` as a `Cow::Owned` key in the underlying `IndexMap`, with `b` as value.
    fn insert(&mut self, line: &[u8], b: Bookkeeping);

    /// Insert `line` as a `Cow::Owned` key in the underlying `IndexMap`, with `b` as value.
    fn insert_borrowed(&mut self, line: &'data [u8], b: Bookkeeping);

    fn insert_borrowed_lines(&mut self, mut slice: &'data [u8], b: Bookkeeping) {
        while let Some(end) = memchr(b'\n', slice) {
            let (mut line, rest) = slice.split_at(end);
            slice = &rest[1..];
            if let Some(&maybe_cr) = line.last() {
                if maybe_cr == b'\r' {
                    line = &line[..line.len() - 1];
                }
            }
            self.insert_borrowed(line, b);
        }
        if !slice.is_empty() {
            self.insert_borrowed(slice, b);
        }
    }

    /// Sometimes we need to update the bookkeeping information
    fn get_mut(&mut self, line: &[u8]) -> Option<&mut Bookkeeping>;

    /// `IndexMap`'s `.retain` method is `O(n)` and preserves the order of the
    /// keys, so it's safe to expose it. We don't expose `.remove`, because it
    /// doesn't preserve key order, and we don't expose `.shift_remove`, which
    /// does preserve order, because `.shift_remove` is *also* `O(n)`, and using
    /// it to remove elements one by one means `O(n^2)` performance.
    fn retain(&mut self, keep: impl FnMut(&mut Bookkeeping) -> bool);

    /// Output the `UncountedSet`'s lines with the appropriate Byte Order Mark and line
    /// terminator.
    fn output_to(&self, out: impl io::Write) -> Result<()>;
}
impl<'data, Bookkeeping: Copy> ZetSet<'data, Bookkeeping> for UncountedSet<'data, Bookkeeping> {
    fn insert(&mut self, line: &[u8], b: Bookkeeping) {
        self.set.insert(Cow::from(line.to_vec()), b);
    }

    fn insert_borrowed(&mut self, line: &'data [u8], b: Bookkeeping) {
        self.set.insert(Cow::Borrowed(line), b);
    }

    /// Sometimes we need to update the bookkeeping information
    fn get_mut(&mut self, line: &[u8]) -> Option<&mut Bookkeeping> {
        self.set.get_mut(line)
    }

    /// `IndexMap`'s `.retain` method is `O(n)` and preserves the order of the
    /// keys, so it's safe to expose it. We don't expose `.remove`, because it
    /// doesn't preserve key order, and we don't expose `.shift_remove`, which
    /// does preserve order, because `.shift_remove` is *also* `O(n)`, and using
    /// it to remove elements one by one means `O(n^2)` performance.
    fn retain(&mut self, mut keep: impl FnMut(&mut Bookkeeping) -> bool) {
        self.set.retain(|_k, v| keep(v));
    }

    /// Output the `UncountedSet`'s lines with the appropriate Byte Order Mark and line
    /// terminator.
    fn output_to(&self, mut out: impl io::Write) -> Result<()> {
        out.write_all(self.bom)?;
        for line in self.set.keys() {
            out.write_all(line)?;
            out.write_all(self.line_terminator)?;
        }
        out.flush()?;
        Ok(())
    }
}

pub(crate) trait ToUncountedSet<'data> {
    fn to_uncounted_set_with<Bookkeeping: Copy>(
        &'data self,
        b: Bookkeeping,
    ) -> UncountedSet<'data, Bookkeeping>;
}

impl<'data> ToUncountedSet<'data> for &[u8] {
    /// `slice.to_uncounted_set_with(b)` takes a byte slice (`&[u8]`) with multiple
    /// lines and returns a `UncountedSet` with line terminator and Byte Order Mark
    /// (or empty string) taken from the slice, and with each line of the slice
    /// represented in the set by a `Cow::Borrowed` key with the bookkeeping
    /// value `b`.
    fn to_uncounted_set_with<Bookkeeping: Copy>(&self, b: Bookkeeping) -> UncountedSet<Bookkeeping> {
        let (bom, line_terminator) = output_info(self);
        let all_lines = &self[bom.len()..];

        let mut set = UncountedSet { set: CowSet::default(), bom, line_terminator };
        set.insert_borrowed_lines(all_lines, b);
        set
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

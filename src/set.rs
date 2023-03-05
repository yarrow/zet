//! Provides the `ZetSet` trait, implemented by `UncountedSet` and `CountedSet`.
//! A `ZetSet` is intended to be initialized from the contents of the first
//! input file.
use anyhow::Result;
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use memchr::memchr;
use std::borrow::Cow;
use std::io;

pub(crate) trait Bookkeeping: Copy + PartialEq {}
impl<T: Copy + PartialEq> Bookkeeping for T {}

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
pub(crate) struct UncountedSet<'data, B: Bookkeeping> {
    set: CowSet<'data, B>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

pub(crate) trait ZetSet<'data, B: Bookkeeping> {
    /// Insert `line` as a `Cow::Owned` key in the underlying `IndexMap`, with `b` as value.
    fn insert(&mut self, line: &[u8], b: B);

    /// Insert `line` as a `Cow::Owned` key in the underlying `IndexMap`, with `b` as value.
    fn insert_borrowed(&mut self, line: &'data [u8], b: B);

    fn insert_borrowed_lines(&mut self, mut slice: &'data [u8], b: B) {
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

    /// Set the bookkeeping information, only if the key is already present
    fn change_if_present(&mut self, line: &[u8], b: B);

    /// If `line` isn't present, insert it with `new` as bookkeeping; otherwise
    /// change bookkeeping to `change`. (This is way too tightly coupled to the
    /// `SingleByFile` / `MultipleByFile` code!)
    fn ensure_unique_source(&mut self, line: &[u8], new: B, change: B);

    /// `IndexMap`'s `.retain` method is `O(n)` and preserves the order of the
    /// keys, so it's safe to expose it. We don't expose `.remove`, because it
    /// doesn't preserve key order, and we don't expose `.shift_remove`, which
    /// does preserve order, because `.shift_remove` is *also* `O(n)`, and using
    /// it to remove elements one by one means `O(n^2)` performance.
    fn retain(&mut self, keep: impl Fn(B) -> bool);

    /// Output the `UncountedSet`'s lines with the appropriate Byte Order Mark and line
    /// terminator.
    fn output_to(&self, out: impl io::Write) -> Result<()>;
}

impl<'data, B: Bookkeeping> ZetSet<'data, B> for UncountedSet<'data, B> {
    fn insert(&mut self, line: &[u8], b: B) {
        self.set.insert(Cow::from(line.to_vec()), b);
    }

    fn insert_borrowed(&mut self, line: &'data [u8], b: B) {
        self.set.insert(Cow::Borrowed(line), b);
    }

    fn change_if_present(&mut self, line: &[u8], b: B) {
        if let Some(v) = self.set.get_mut(line) {
            *v = b;
        }
    }

    fn ensure_unique_source(&mut self, line: &[u8], pristine: B, erase: B) {
        match self.set.get_mut(line) {
            None => self.insert(line, pristine),
            Some(old) => {
                if *old != pristine {
                    *old = erase
                }
            }
        }
    }

    fn retain(&mut self, keep: impl Fn(B) -> bool) {
        self.set.retain(|_k, v| keep(*v));
    }

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

struct Counted<B: Bookkeeping> {
    count: u32,
    b: B,
}
pub(crate) struct CountedSet<'data, B: Bookkeeping> {
    set: CowSet<'data, Counted<B>>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
impl<'data, B: Bookkeeping> CountedSet<'data, B> {
    pub fn retain_single(&mut self) {
        self.set.retain(|_k, v| v.count == 1);
    }
    pub fn retain_multiple(&mut self) {
        self.set.retain(|_k, v| v.count > 1);
    }
}

fn seen_once<B: Bookkeeping>(b: B) -> Counted<B> {
    Counted::<B> { count: 1, b }
}

impl<'data, B: Bookkeeping> ZetSet<'data, B> for CountedSet<'data, B> {
    fn insert(&mut self, line: &[u8], b: B) {
        self.set
            .entry(Cow::from(line.to_vec()))
            .and_modify(|v| v.count = v.count.saturating_add(1))
            .or_insert_with(|| seen_once(b));
    }
    fn insert_borrowed(&mut self, line: &'data [u8], b: B) {
        self.set
            .entry(Cow::Borrowed(line))
            .and_modify(|v| v.count = v.count.saturating_add(1))
            .or_insert_with(|| seen_once(b));
    }

    fn change_if_present(&mut self, line: &[u8], b: B) {
        if let Some(v) = self.set.get_mut(line) {
            v.b = b;
        }
    }

    fn ensure_unique_source(&mut self, line: &[u8], pristine: B, erase: B) {
        match self.set.get_mut(line) {
            None => self.insert(line, pristine),
            Some(old) => {
                old.count = old.count.saturating_add(1);
                if old.b != pristine {
                    old.b = erase
                }
            }
        }
    }
    fn output_to(&self, mut out: impl io::Write) -> Result<()> {
        out.write_all(self.bom)?;
        for line in self.set.keys() {
            out.write_all(line)?;
            out.write_all(self.line_terminator)?;
        }
        out.flush()?;
        Ok(())
    }
    fn retain(&mut self, keep: impl Fn(B) -> bool) {
        self.set.retain(|_k, v| keep(v.b));
    }
}

/// `slice.to_[un]counted_set_with(b)` takes a byte slice (`&[u8]`) with
/// multiple lines and returns a `ZetSet` (`CountedSet` or `UncountedSet`) with
/// line terminator and Byte Order Mark (or empty string) taken from the slice,
/// and with each line of the slice represented in the set by a `Cow::Borrowed`
/// key with the bookkeeping value `b`.
pub(crate) trait ToUncountedSet<'data> {
    fn to_uncounted_set_with<B: Bookkeeping>(&'data self, b: B) -> UncountedSet<'data, B>;
}
pub(crate) trait ToCountedSet<'data> {
    fn to_counted_set_with<B: Bookkeeping>(&'data self, b: B) -> CountedSet<'data, B>;
}
impl<'data> ToUncountedSet<'data> for &[u8] {
    fn to_uncounted_set_with<B: Bookkeeping>(&self, b: B) -> UncountedSet<B> {
        let (bom, line_terminator) = output_info(self);
        let all_lines = &self[bom.len()..];

        let mut set = UncountedSet { set: CowSet::default(), bom, line_terminator };
        set.insert_borrowed_lines(all_lines, b);
        set
    }
}
impl<'data> ToCountedSet<'data> for &[u8] {
    fn to_counted_set_with<B: Bookkeeping>(&self, b: B) -> CountedSet<B> {
        let (bom, line_terminator) = output_info(self);
        let all_lines = &self[bom.len()..];

        let mut set = CountedSet { set: CowSet::default(), bom, line_terminator };
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

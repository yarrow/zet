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
/// * Optionally, the `ZetSet` can also track the number of times each line occurs
pub(crate) struct ZetSet<'data, Item: Copy, Counter: Tally> {
    set: CowSet<'data, Bookkeeping<Item, Counter>>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

/// The `Bookkeeping` struct combines the `Item` used for a set operation with
/// the `Counter` used to count (or ignore) the number of times a line has
/// occurred.
#[derive(Clone, Copy)]
pub(crate) struct Bookkeeping<Item: Copy, Counter: Tally> {
    item: Item,
    count: Counter,
}

/// The `Tally` trait is used for counting the number of times a line is
/// inserted in a `ZetSet`.  (Or, optionally, not to count that.)
pub(crate) trait Tally: Copy {
    fn new() -> Self;
    fn value(self) -> u32;
    fn increment(&mut self);
}

/// The `Counted` flavor of `Tally` actually counts things. Its value is never
/// zero.
pub(crate) type Counted = u32;
impl Tally for Counted {
    fn new() -> Self {
        1
    }
    fn value(self) -> u32 {
        self
    }
    fn increment(&mut self) {
        *self += 1
    }
}

/// The `Uncounted` flavor of `Tally` has a `value()` of zero no matter how many
/// times you `increment()` it.
#[derive(Clone, Copy)]
pub(crate) struct Uncounted();
impl Tally for Uncounted {
    fn new() -> Self {
        Uncounted()
    }
    fn value(self) -> u32 {
        0
    }
    fn increment(&mut self) {}
}

/// Creates a new `ZetSet`, with each key a line borrowed from `slice`, and value
/// `Bookkeeping{item, count}` for every line.
pub(crate) fn zet_set_from<Item: Copy, Counter: Tally>(
    mut slice: &[u8],
    item: Item,
    count: Counter,
) -> ZetSet<Item, Counter> {
    let (bom, line_terminator) = output_info(slice);
    slice = &slice[bom.len()..];
    let mut zet = ZetSet { set: CowSet::default(), bom, line_terminator };
    zet.insert_borrowed_lines(slice, Bookkeeping { item, count });
    zet
}

impl<'data, Counter: Tally, Item: Copy> ZetSet<'data, Item, Counter> {
    /// Insert `line` as `Cow::Owned` to the underlying `IndexMap`
    pub(crate) fn insert(&mut self, line: &[u8], item: Item) {
        self.set
            .entry(Cow::from(line.to_vec()))
            .and_modify(|v| v.count.increment())
            .or_insert(Bookkeeping { item, count: Counter::new() });
    }

    /// Insert `line` as `Cow::Borrowed` to the underlying `IndexMap`
    fn insert_borrowed(&mut self, line: &'data [u8], item: Bookkeeping<Item, Counter>) {
        self.set.entry(Cow::Borrowed(line)).and_modify(|v| v.count.increment()).or_insert(item);
    }

    /// Insert every line in `slice`
    fn insert_borrowed_lines(&mut self, mut slice: &'data [u8], item: Bookkeeping<Item, Counter>) {
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

    /// Sometimes we need to update the bookkeeping information
    /// We expose only the `item` field, not the `count` field
    pub(crate) fn get_mut(&mut self, line: &[u8]) -> Option<&mut Item> {
        self.set.get_mut(line).map(|v| &mut v.item)
    }

    /// Like `IndexMap`'s `.retain` method, but exposes just the item, and by value.
    pub(crate) fn retain(&mut self, keep: impl Fn(Item) -> bool) {
        self.set.retain(|_k, v| keep(v.item));
    }

    /// Retain lines seen just once
    pub(crate) fn retain_single(&mut self) {
        self.set.retain(|_k, v| v.count.value() == 1);
    }

    /// Retain lines seen more than once
    pub(crate) fn retain_multiple(&mut self) {
        self.set.retain(|_k, v| v.count.value() > 1);
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

    /// Output the `ZetSet`'s lines with the appropriate Byte Order Mark and line
    /// terminator.
    pub(crate) fn output_with_count_to(&self, mut out: impl io::Write) -> Result<()> {
        let Some(max_count) = self.set.values().map(|v| v.count.value()).max() else { return Ok(()) };
        let width = (max_count.ilog10() + 1) as usize;
        out.write_all(self.bom)?;
        for (line, info) in self.set.iter() {
            write!(out, "{:width$} ", info.count.value())?;
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

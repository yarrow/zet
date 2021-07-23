//! Input/Output structs and functions
use anyhow::{Context, Result};
use bstr::io::BufReadExt;
use encoding_rs_io::{DecodeReaderBytes, DecodeReaderBytesBuilder};
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use memchr::memchr;
use std::borrow::Cow;
use std::{
    fs,
    fs::File,
    io,
    io::BufReader,
    ops::FnMut,
    path::{Path, PathBuf},
};

/// Return the contents of the first file named in `files` as a Vec<u8>, and an iterator over the
/// subsequent arguments.
pub(crate) fn first_and_rest(files: &[PathBuf]) -> Option<(Result<Vec<u8>>, RemainingOperands)> {
    match files {
        [] => None,
        [first, rest @ ..] => {
            let first_operand = fs::read(&first)
                .with_context(|| format!("Can't read file: {}", first.display()))
                .map(decode_if_utf16);
            Some((first_operand, RemainingOperands::from(rest.to_vec())))
        }
    }
}

/// Decode UTF-16 to UTF-8 if we see a UTF-16 Byte Order Mark at the beginning of `candidate`.
/// Otherwise return `candidate` unchanged
fn decode_if_utf16(candidate: Vec<u8>) -> Vec<u8> {
    // Translate UTF16 to UTF8
    // Note: `decode_without_bom_handling` will change malformed sequences to the
    // Unicode REPLACEMENT CHARACTER. Should we report an error instead?
    //
    // "with BOM handling" means that the UTF-16 BOM is translated to a UTF-8 BOM
    //
    if let Some((enc, _)) = encoding_rs::Encoding::for_bom(&candidate) {
        if [encoding_rs::UTF_16LE, encoding_rs::UTF_16BE].contains(&enc) {
            let (translated, _had_malformed_sequences) =
                enc.decode_without_bom_handling(&candidate);
            return translated.into_owned().into_bytes();
        }
    }
    return candidate;
}

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
pub(crate) struct ZetSet<'data, Bookkeeping: Copy> {
    set: CowSet<'data, Bookkeeping>,
    bom: &'static [u8],             // Byte Order Mark or empty
    line_terminator: &'static [u8], // \n or \r\n
}
type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

impl<'data, Bookkeeping: Copy> ZetSet<'data, Bookkeeping> {
    /// Insert `line` as `Cow::Owned` to the underlying `IndexMap`
    pub(crate) fn insert(&mut self, line: &[u8], b: Bookkeeping) {
        self.set.insert(Cow::from(line.to_vec()), b);
    }

    /// Sometimes we need to update the bookkeeping information
    pub(crate) fn get_mut(&mut self, line: &[u8]) -> Option<&mut Bookkeeping> {
        self.set.get_mut(line)
    }

    /// `IndexMap`'s `.retain` method is `O(n)` and preserves the order of the
    /// keys, so it's safe to expose it. We don't expose `.remove`, because it
    /// doesn't preserve key order, and we don't expose `.shift_remove`, which
    /// does preserve order, because `.shift_remove` is *also* `O(n)`, and using
    /// it to remove elements one by one means `O(n^2)` performance.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&mut Bookkeeping) -> bool) {
        self.set.retain(|_k, v| keep(v));
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

pub(crate) trait ToZetSet<'data> {
    fn to_zet_set_with<Bookkeeping: Copy>(
        &'data self,
        b: Bookkeeping,
    ) -> ZetSet<'data, Bookkeeping>;
}

impl<'data> ToZetSet<'data> for &[u8] {
    /// `slice.to_zet_set_with(b)` takes a byte slice (`&[u8]`) with multiple
    /// lines and returns a `ZetSet` with line terminator and Byte Order Mark
    /// (or empty string) taken from the slice, and with each line of the slice
    /// represented in the set by a `Cow::Borrowed` key with the bookkeeping
    /// value `b`.
    fn to_zet_set_with<Bookkeeping: Copy>(&self, b: Bookkeeping) -> ZetSet<Bookkeeping> {
        let (bom, line_terminator) = output_info(self);
        let all_lines = &self[bom.len()..];

        let set = borrowed_map_of(all_lines, b);

        ZetSet { set, bom, line_terminator }
    }
}

/// Returns `(bom, line_terminator)`, where `bom` is the (UTF-8) Byte Order
/// Mark, or the empty string if `slice` has none, and `line_terminator` is
/// `\r\n` if the first line of `slice` ends with `\r\n`, and `\n` if the first
/// line ends just with '\n` (or is the only line in the file and has no line
/// terminator).
fn output_info(slice: &[u8]) -> (&'static [u8], &'static [u8]) {
    let mut bom: &'static [u8] = b"";
    let mut line_terminator: &'static [u8] = b"\n";
    if has_bom(slice) {
        bom = BOM_BYTES;
    }
    if let Some(n) = memchr(b'\n', slice) {
        if n > 0 && slice[n - 1] == b'\r' {
            line_terminator = b"\r\n"
        }
    }
    (bom, line_terminator)
}

/// Returns a `CowSet` with every line of `slice` inserted as a `Cow::Borrowed`
/// key with bookkeeping value `b`
fn borrowed_map_of<Bookkeeping: Copy>(mut slice: &[u8], b: Bookkeeping) -> CowSet<Bookkeeping> {
    let mut set = CowSet::default();
    while let Some(end) = memchr(b'\n', slice) {
        let (mut line, rest) = slice.split_at(end);
        slice = &rest[1..];
        if let Some(&maybe_cr) = line.last() {
            if maybe_cr == b'\r' {
                line = &line[..line.len() - 1];
            }
        }
        set.insert(Cow::Borrowed(line), b);
    }
    if !slice.is_empty() {
        set.insert(Cow::Borrowed(slice), b);
    }
    set
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

/// The first operand is read into memory in its entirety, but that's not
/// efficient for the second and subsequent operands.  The `RemainingOperands`
/// structure is an iterator over those operands.
pub(crate) struct RemainingOperands {
    files: std::vec::IntoIter<PathBuf>,
}

impl From<Vec<PathBuf>> for RemainingOperands {
    fn from(files: Vec<PathBuf>) -> Self {
        RemainingOperands { files: files.into_iter() }
    }
}

impl Iterator for RemainingOperands {
    type Item = Result<NextOperand>;
    fn next(&mut self) -> Option<Self::Item> {
        self.files.next().map(|path| reader_for(&path))
    }
}

/// `NextOperand` is the `Item` type for the `RemainingOperands` iterator. The
/// `reader` field is a reader for the file with path `path`. We keep the `path`
/// field around to improve error messages.
pub(crate) struct NextOperand {
    path: PathBuf,
    reader: BufReader<DecodeReaderBytes<File, Vec<u8>>>,
}

/// The reader for a second or subsequent operand is a buffered reader with the
/// ability to decode UTF-16 files.
fn reader_for(path: &Path) -> Result<NextOperand> {
    let f = File::open(path).with_context(|| format!("Can't open file: {}", path.display()))?;
    let reader = BufReader::with_capacity(
        32 * 1024,
        DecodeReaderBytesBuilder::new()
            .bom_sniffing(true) // Look at the BOM to detect UTF-16 files and convert to UTF-8
            .strip_bom(true) // Remove the BOM before sending data to us
            .utf8_passthru(true) // Don't enforce UTF-8 (BOM or no BOM)
            .build(f),
    );
    Ok(NextOperand { path: path.to_owned(), reader })
}
impl NextOperand {
    /// A convenience wrapper around `bstr::for_byte_line`
    pub(crate) fn for_byte_line<F>(self, mut for_each_line: F) -> Result<()>
    where
        F: FnMut(&[u8]),
    {
        let complaint = format!("Error reading file: {}", self.path.display());
        self.reader
            .for_byte_line(|line| {
                for_each_line(line);
                Ok(true)
            })
            .context(complaint)?;
        Ok(())
    }
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

    fn utf_16le(source: &str) -> Vec<u8> {
        let mut result = b"\xff\xfe".to_vec();
        for b in source.as_bytes().iter() {
            result.push(*b);
            result.push(0);
        }
        result
    }

    fn utf_16be(source: &str) -> Vec<u8> {
        let mut result = b"\xfe\xff".to_vec();
        for b in source.as_bytes().iter() {
            result.push(0);
            result.push(*b);
        }
        result
    }

    fn abominate(expected: &str) -> String {
        UTF8_BOM.to_string() + expected
    }

    #[test]
    fn utf_16le_is_translated_to_utf8() {
        let expected = "The cute red crab\n jumps over the lazy blue gopher\n";
        assert_eq!(decode_if_utf16(utf_16le(&expected)), abominate(expected).as_bytes());
    }

    #[test]
    fn utf_16be_is_translated_to_utf8() {
        let expected = "The cute red crab\n jumps over the lazy blue gopher\n";
        assert_eq!(decode_if_utf16(utf_16be(&expected)), abominate(expected).as_bytes());
    }
}

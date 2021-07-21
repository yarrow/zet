//! Input/Output structs and functions
use crate::CowSet;
use anyhow::{Context, Result};
use bstr::io::BufReadExt;
use encoding_rs_io::{DecodeReaderBytes, DecodeReaderBytesBuilder};
use std::borrow::Cow;
use std::{
    fs,
    fs::File,
    io,
    io::BufReader,
    ops::FnMut,
    path::{Path, PathBuf},
};

pub(crate) fn first_and_rest(files: &[PathBuf]) -> Option<(Result<Vec<u8>>, ContentsIter)> {
    match files {
        [] => None,
        [first, rest @ ..] => {
            let attempt = fs::read(&first).with_context(|| path_context(&first));
            let first_operand = attempt.map(decode_if_utf16);
            Some((first_operand, ContentsIter::from(rest.to_vec())))
        }
    }
}

fn path_context(path: &Path) -> String {
    format!("Can't read file: {}", path.display())
}

pub(crate) fn zet_set_from<Bookkeeping: Copy>(
    first_operand: &[u8],
    b: Bookkeeping,
) -> ZetSet<Bookkeeping> {
    let (bom, line_terminator) = output_info(first_operand);
    let first_operand = &first_operand[bom.len()..];

    let set = borrowed_map_of(first_operand, b);

    ZetSet { bom, line_terminator, set }
}

fn output_info(contents: &[u8]) -> (&'static [u8], &'static [u8]) {
    let mut bom: &'static [u8] = b"";
    let mut line_terminator: &'static [u8] = b"\n";
    if has_bom(contents) {
        bom = BOM_BYTES;
    }
    if let Some(n) = memchr(b'\n', contents) {
        if n > 0 && contents[n - 1] == b'\r' {
            line_terminator = b"\r\n"
        }
    }
    (bom, line_terminator)
}

fn borrowed_map_of<Bookkeeping: Copy>(mut contents: &[u8], b: Bookkeeping) -> CowSet<Bookkeeping> {
    let mut set = CowSet::default();
    while let Some(end) = memchr(b'\n', contents) {
        let (mut line, rest) = contents.split_at(end);
        contents = &rest[1..];
        if let Some(&maybe_cr) = line.last() {
            if maybe_cr == b'\r' {
                line = &line[..line.len() - 1];
            }
        }
        set.insert(Cow::Borrowed(line), b);
    }
    if !contents.is_empty() {
        set.insert(Cow::Borrowed(contents), b);
    }
    set
}

pub(crate) struct ZetSet<'data, Bookkeeping: Copy> {
    bom: &'static [u8],
    line_terminator: &'static [u8],
    set: CowSet<'data, Bookkeeping>,
}

impl<'data, Bookkeeping: Copy> ZetSet<'data, Bookkeeping> {
    pub(crate) fn insert(&mut self, line: &[u8], b: Bookkeeping) {
        self.set.insert(Cow::from(line.to_vec()), b);
    }
    pub(crate) fn get_mut(&mut self, line: &[u8]) -> Option<&mut Bookkeeping> {
        self.set.get_mut(line)
    }
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&mut Bookkeeping) -> bool) {
        self.set.retain(|_k, v| keep(v));
    }
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
const BOM_0: u8 = b'\xEF';
const BOM_1: u8 = b'\xBB';
const BOM_2: u8 = b'\xBF';
const BOM_BYTES: &[u8] = b"\xEF\xBB\xBF";
fn has_bom(contents: &[u8]) -> bool {
    contents.len() >= 3 && contents[0] == BOM_0 && contents[1] == BOM_1 && contents[2] == BOM_2
}

type NextOperand = BufReader<DecodeReaderBytes<File, Vec<u8>>>;
pub(crate) struct SubsequentOperand {
    path: PathBuf,
    reader: io::Result<NextOperand>,
}
fn maybe_decoded(path: &Path) -> SubsequentOperand {
    let reader = File::open(path).map(|f| {
        BufReader::with_capacity(
            32 * 1024,
            DecodeReaderBytesBuilder::new()
                .bom_sniffing(true)
                .strip_bom(true)
                .utf8_passthru(true)
                .build(f),
        )
    });
    SubsequentOperand { path: path.to_owned(), reader }
}
impl SubsequentOperand {
    pub(crate) fn for_byte_line<F>(self, for_each_line: F) -> Result<()>
    where
        F: FnMut(&[u8]),
    {
        let complaint = format!("Error processing file {}", self.path.display());
        f_b_line(self.reader, for_each_line).context(complaint)?;
        Ok(())
    }
}
fn f_b_line<F>(reader: io::Result<NextOperand>, mut for_each_line: F) -> Result<()>
where
    F: FnMut(&[u8]),
{
    reader?.for_byte_line(|line| {
        for_each_line(line);
        Ok(true)
    })?;
    Ok(())
}

pub(crate) struct ContentsIter {
    files: std::vec::IntoIter<PathBuf>,
}

impl From<Vec<PathBuf>> for ContentsIter {
    fn from(files: Vec<PathBuf>) -> Self {
        ContentsIter { files: files.into_iter() }
    }
}

impl Iterator for ContentsIter {
    type Item = SubsequentOperand;
    fn next(&mut self) -> Option<Self::Item> {
        self.files.next().map(|path| maybe_decoded(&path))
    }
}

use memchr::memchr;

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

pub(crate) struct InputLines<'data> {
    remaining: &'data [u8],
}

/// An iterator over the lines of the file contents, without line terminators.
/// That is, from each line we strip `\r\n` or `\n`, whichever is longest.
impl<'data> Iterator for InputLines<'data> {
    type Item = &'data [u8];
    fn next(&mut self) -> Option<Self::Item> {
        match memchr(b'\n', self.remaining) {
            None => {
                if self.remaining.is_empty() {
                    None
                } else {
                    // last line doesn't end with `\n`
                    let line = self.remaining;
                    self.remaining = b"";
                    Some(line)
                }
            }
            Some(mut end) => {
                let restart = end + 1;
                if end > 0 && self.remaining[end - 1] == b'\r' {
                    end -= 1
                }
                let line = &self.remaining[..end];
                self.remaining = &self.remaining[restart..];
                Some(line)
            }
        }
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

//! Input/Output structs and functions
use anyhow::{Context, Result};
use std::{fs, io, path::PathBuf};

/// Returns a triple consisting of:
///
/// * The contents of the first file in `files`, or `None` if there were no files;
/// * An iterator over the (contents of) the remaing files;
/// * A `SetWriter` that knows
///     * whether or not to output a Byte Order Mark at the start of output, and
///     * whether to end each line with `\r\n` or just '\n`.
pub fn prepare(files: Vec<PathBuf>) -> Result<(Option<Vec<u8>>, ContentsIter, SetWriter)> {
    let mut rest = ContentsIter::from(files);
    let first = rest.next();
    match first {
        None => Ok((None, rest, SetWriter { bom: b"", eol: b"" })),
        Some(Err(e)) => Err(e),
        Some(Ok(first)) => {
            let mut eol: &[u8] = b"\n";
            if let Some(n) = memchr(b'\n', &first) {
                if n > 0 && first[n - 1] == b'\r' {
                    eol = b"\r\n";
                }
            }
            let bom = if has_bom(&first) { BOM_BYTES } else { b"" };
            Ok((Some(first), rest, SetWriter { bom, eol }))
        }
    }
}

/// Remember whether the first file had a BOM, and whether lines should end with `\r\n` or `\n`
#[derive(Debug)]
pub struct SetWriter {
    bom: &'static [u8],
    eol: &'static [u8],
}

impl SetWriter {
    /// Write the result of `do_calculation` to stdout, buffered if not going to the terminal
    /// and locked in any case.
    pub fn output(&self, result: crate::LineIterator) -> Result<()> {
        if atty::is(atty::Stream::Stdout) {
            self.inner(result, io::stdout().lock())
        } else {
            self.inner(result, io::BufWriter::new(io::stdout().lock()))
        }
    }
    fn inner(&self, result: crate::LineIterator, mut out: impl io::Write) -> Result<()> {
        out.write_all(self.bom)?;
        for line in result {
            out.write_all(line)?;
            out.write_all(self.eol)?;
        }
        out.flush()?;
        Ok(())
    }
}

/// Given a list of file paths (as a vector of `PathBuf`s), iterates over their contents. We
/// guarantee that each non-empty file's contents ends with `\n` (and with `\r\n` if the file's
/// penultimate line ends with `\r\n`).
///
/// If `files` is a `Vec<PathBuf>`, then `ContentsIter::from(files)` returns an iterator over the
/// *contents* of the given `files`, decoded from UTF-16 to UTF-8 if a UTF-16 Byte Order Mark is
/// detected.  If we want to print out the (entire!) contents of a file if it contains a `b'Z'`
/// anywhere, we could use
///
/// ```no_run
/// # use anyhow::Result;
/// # fn main() -> Result<()> {
/// use std::{io::stdout, io::Write, path::PathBuf};
/// use zet::io::ContentsIter;
///
/// let files = vec![PathBuf::from("a.txt"), PathBuf::from("b.txt"), PathBuf::from("c.txt")];
/// for result in ContentsIter::from(files) {
///     let contents = result?;
///     if contents.contains(&b'Z') {
///         stdout().write(&contents);
///     }
///  }
///  # Ok(())
///  # }
/// ```
pub struct ContentsIter {
    files: std::vec::IntoIter<PathBuf>,
}
impl From<Vec<PathBuf>> for ContentsIter {
    fn from(files: Vec<PathBuf>) -> Self {
        ContentsIter { files: files.into_iter() }
    }
}
impl Iterator for ContentsIter {
    type Item = Result<Vec<u8>>;
    fn next(&mut self) -> Option<Self::Item> {
        let path = self.files.next()?;
        let attempt =
            fs::read(&path).with_context(|| format!("Can't read file: {}", path.to_string_lossy()));
        Some(match attempt {
            Ok(contents) => Ok(decode_if_utf16(contents)),
            Err(err) => Err(err),
        })
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

const BOM_0: u8 = b'\xEF';
const BOM_1: u8 = b'\xBB';
const BOM_2: u8 = b'\xBF';
const BOM_BYTES: &[u8] = b"\xEF\xBB\xBF";
pub(crate) fn has_bom(contents: &[u8]) -> bool {
    contents.len() >= 3 && contents[0] == BOM_0 && contents[1] == BOM_1 && contents[2] == BOM_2
}

pub(crate) fn lines_of(contents: &[u8]) -> InputLines {
    if has_bom(contents) {
        InputLines { remaining: &contents[3..] }
    } else {
        InputLines { remaining: contents }
    }
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

    #[test]
    fn fn_lines_of_strips_utf8_bom_and_line_terminators() {
        let with_bom = UTF8_BOM.to_string() + "abc\ndefg\nxyz\n";
        let expected: Vec<&[u8]> = vec![b"abc", b"defg", b"xyz"];
        let result = lines_of(with_bom.as_bytes()).collect::<Vec<_>>();
        assert_eq!(expected, result);
    }
}

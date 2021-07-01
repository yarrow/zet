//! Input/Output structs and functions
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Returns a triple consisting of:
///
/// * The contents of the first file in `files`, or `None` if there were no files;
/// * An iterator over the (contents of) the remaing files;
/// * A `SetWriter` that knows
///     * whether or not to output a Byte Order Mark at the start of output, and
///     * whether to end each line with `\r\n` or just '\n`.
pub fn prepare(
    files: Vec<PathBuf>,
) -> Result<(Option<Vec<u8>>, ContentsIter, SetWriter), failure::Error> {
    let mut rest = ContentsIter::from(files);
    let first = rest.next();
    match first {
        None => Ok((None, rest, SetWriter { bom: b"" })),
        Some(Err(e)) => Err(e),
        Some(Ok(first)) => {
            let bom = if has_bom(&first) { BOM_BYTES } else { b"" };
            Ok((Some(first), rest, SetWriter { bom }))
        }
    }
}

/// Remember whether the first file had a BOM, and whether lines should end with `\r\n` or `\n`
#[derive(Debug)]
pub struct SetWriter {
    bom: &'static [u8],
}

impl SetWriter {
    /// Write the result of `do_calculation` to stdout, buffered if not going to the terminal
    /// and locked in any case.
    pub fn output(&self, result: crate::LineIterator) -> Result<(), failure::Error> {
        if atty::is(atty::Stream::Stdout) {
            self.inner(result, io::stdout().lock())
        } else {
            self.inner(result, io::BufWriter::new(io::stdout().lock()))
        }
    }
    fn inner(
        &self,
        result: crate::LineIterator,
        mut out: impl io::Write,
    ) -> Result<(), failure::Error> {
        out.write_all(self.bom)?;
        for line in result {
            out.write_all(line)?;
        }
        out.flush()?;
        Ok(())
    }
}

/// Given a list of file paths (as a vector of `PathBuf`s), iterates over their contents.
/// We guarantee that each non-empty file's contents ends with `\n` (and with `\r\n` if the
/// file's penultimate line ends with `\r\n`).
///
/// If `files` is a `Vec<PathBuf>`, then `ContentsIter::from(files)` returns an iterator over
/// the *contents* of the given `files`.  If we want to print out the (entire!) contents of
/// a file if it contains a `b'Z'` anywhere, we could use
///
/// ```no_run
/// # fn main() -> Result<(), failure::Error> {
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
    type Item = Result<Vec<u8>, failure::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let path = self.files.next()?;
        Some(match read_file_and_adjust(&path) {
            Ok(contents) => Ok(contents),
            Err(io_err) => {
                let path = path.to_string_lossy();
                Err(format_err!("Can't read file `{}`: {}", path, io_err))
            }
        })
    }
}

use memchr::memchr;

fn read_and_adjust(
    source: &mut impl io::Read,
    initial_buffer_size: usize,
) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes: Vec<u8> = Vec::with_capacity(initial_buffer_size);
    source.read_to_end(&mut bytes)?;

    // Translate UTF16 to UTF8
    // Note: `decode_without_bom_handling` will change malformed sequences to the
    // Unicode REPLACEMENT CHARACTER. Should we report an error instead?
    //
    // "with BOM handling" means that the UTF-16 BOM is translated to a UTF-8 BOM
    //
    if let Some((enc, _)) = encoding_rs::Encoding::for_bom(&bytes) {
        if [encoding_rs::UTF_16LE, encoding_rs::UTF_16BE].contains(&enc) {
            let (new_bytes, _had_malformed_sequences) = enc.decode_without_bom_handling(&bytes);
            bytes = new_bytes.into_owned().into_bytes();
        }
    }

    // If the last line has no end-of-line marker (either `\r\n` or `\n`), then use the first
    // line's marker. (Or '\n' if there is just one line and it has no marker.)
    match &bytes.last() {
        None | Some(b'\n') => {}
        _ => {
            if let Some(n) = memchr(b'\n', &bytes) {
                if n > 0 && bytes[n - 1] == b'\r' {
                    bytes.push(b'\r')
                }
            }
            bytes.push(b'\n')
        }
    }
    Ok(bytes)
}

/// The following function is based on `std::fs::read` — we can't use
/// `fs::read` directly, because we want to allocate *two* extra bytes
/// (to add `\r\n` if need be), and `fs::read` only allocates one.
pub fn read_file_and_adjust<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, std::io::Error> {
    #[allow(clippy::cast_possible_truncation)]
    fn initial_buffer_size(file: &fs::File) -> usize {
        // Allocate TWO extra bytes so the buffer doesn't need to grow
        // before the final `read` call at the end of the file.
        // Don't worry about `usize` overflow because reading will fail
        // regardless in that case.
        file.metadata().map(|m| m.len() as usize + 2).unwrap_or(0)
    }
    let mut file = std::fs::File::open(path.as_ref())?;
    let size = initial_buffer_size(&file);
    read_and_adjust(&mut file, size)
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

impl<'data> Iterator for InputLines<'data> {
    type Item = &'data [u8];
    fn next(&mut self) -> Option<Self::Item> {
        match memchr(b'\n', self.remaining) {
            None => None,
            Some(end) => {
                let line = &self.remaining[..=end];
                self.remaining = &self.remaining[end + 1..];
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

    fn abominate(expected: &str) -> String {
        UTF8_BOM.to_string() + expected
    }

    #[test]
    fn utf_16le_is_translated_to_utf8() {
        let expected = "The cute red crab\n jumps over the lazy blue gopher\n";
        let utf16 = utf_16le(&expected);
        let mut source = &utf16[..];
        let result = read_and_adjust(&mut source, 100).unwrap();
        assert_eq!(result, abominate(expected).as_bytes());
    }

    fn utf_16be(source: &str) -> Vec<u8> {
        let mut result = b"\xfe\xff".to_vec();
        for b in source.as_bytes().iter() {
            result.push(0);
            result.push(*b);
        }
        result
    }

    #[test]
    fn utf_16be_is_translated_to_utf8() {
        let expected = "The cute red crab\n jumps over the lazy blue gopher\n";
        let utf16 = utf_16be(&expected);
        let mut source = &utf16[..];
        let result = read_and_adjust(&mut source, 100).unwrap();
        assert_eq!(result, abominate(expected).as_bytes());
    }

    #[test]
    fn fn_lines_of_strips_utf8_bom() {
        let with_bom = UTF8_BOM.to_string() + "abc\ndefg\nxyz\n";
        let expected: Vec<&[u8]> = vec![b"abc\n", b"defg\n", b"xyz\n"];
        let result = lines_of(with_bom.as_bytes()).collect::<Vec<_>>();
        assert_eq!(expected, result);
    }

    #[test]
    fn read_and_adjust_adds_the_eol_of_the_first_line_to_every_line() {
        let mut cr_lf: &[u8] = b"a\r\nb";
        assert_eq!(read_and_adjust(&mut cr_lf, 10).unwrap(), b"a\r\nb\r\n");

        let mut lf: &[u8] = b"a\nb";
        assert_eq!(read_and_adjust(&mut lf, 10).unwrap(), b"a\nb\n");

        let mut no_newline: &[u8] = b"b";
        assert_eq!(read_and_adjust(&mut no_newline, 10).unwrap(), b"b\n");

        let mut empty: &[u8] = b"";
        assert_eq!(read_and_adjust(&mut empty, 10).unwrap(), b"");
    }
}

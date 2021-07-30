//! Provides the `first_and_rest` function, which returns a `Vec<u8>` containing
//! the contents of the first operand and an iterator over the remaining
//! operands. *Note:* this different treatment of the first and remaining
//! operands has the unfortunate result of requiring different code paths for
//! translating UTF16 files into UTF8. That currently seems worth the cost.
use anyhow::{Context, Result};
use bstr::io::BufReadExt;
use encoding_rs_io::{DecodeReaderBytes, DecodeReaderBytesBuilder};
use std::{
    fs,
    fs::File,
    io::BufReader,
    ops::FnMut,
    path::{Path, PathBuf},
};

/// Return the contents of the first file named in `files` as a Vec<u8>, and an iterator over the
/// subsequent arguments.
#[must_use]
pub fn first_and_rest(files: &[PathBuf]) -> Option<(Result<Vec<u8>>, Remaining)> {
    match files {
        [] => None,
        [first, rest @ ..] => {
            let first_operand = fs::read(&first)
                .with_context(|| format!("Can't read file: {}", first.display()))
                .map(decode_if_utf16);
            Some((first_operand, Remaining::from(rest.to_vec())))
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

/// The first operand is read into memory in its entirety, but that's not
/// efficient for the second and subsequent operands.  The `Remaining`
/// structure is an iterator over those operands.
pub struct Remaining {
    files: std::vec::IntoIter<PathBuf>,
}

impl From<Vec<PathBuf>> for Remaining {
    fn from(files: Vec<PathBuf>) -> Self {
        Remaining { files: files.into_iter() }
    }
}

impl Iterator for Remaining {
    type Item = Result<NextOperand>;
    fn next(&mut self) -> Option<Self::Item> {
        self.files.next().map(|path| reader_for(&path))
    }
}

/// `NextOperand` is the `Item` type for the `Remaining` iterator. The
/// `reader` field is a reader for the file with path `path`. We keep the `path`
/// field around to improve error messages.
pub struct NextOperand {
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

    fn abominate(expected: &str) -> String {
        UTF8_BOM.to_string() + expected
    }

    fn to_utf_16le(source: &str) -> Vec<u8> {
        let mut result = b"\xff\xfe".to_vec();
        for b in source.as_bytes().iter() {
            result.push(*b);
            result.push(0);
        }
        result
    }

    fn to_utf_16be(source: &str) -> Vec<u8> {
        let mut result = b"\xfe\xff".to_vec();
        for b in source.as_bytes().iter() {
            result.push(0);
            result.push(*b);
        }
        result
    }

    #[test]
    fn utf_16le_is_translated_to_utf8() {
        let expected = "The cute red crab\n jumps over the lazy blue gopher\n";
        assert_eq!(decode_if_utf16(to_utf_16le(expected)), abominate(expected).as_bytes());
    }

    #[test]
    fn utf_16be_is_translated_to_utf8() {
        let expected = "The cute red crab\n jumps over the lazy blue gopher\n";
        assert_eq!(decode_if_utf16(to_utf_16be(expected)), abominate(expected).as_bytes());
    }
}

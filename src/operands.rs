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
pub fn first_and_rest(files: &[PathBuf]) -> Option<(Result<Vec<u8>>, Vec<PathBuf>)> {
    match files {
        [] => None,
        [first, rest @ ..] => {
            let first_operand = fs::read(first)
                .with_context(|| format!("Can't read file: {}", first.display()))
                .map(decode_if_utf16);
            let rest = rest.to_vec();
            Some((first_operand, rest))
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

/// For operands from which one can read lines as bytes
pub trait Operand {
    /// A convenience wrapper around `bstr::for_byte_line`
    fn for_byte_line<F>(&self, for_each_line: F) -> Result<()> where F: FnMut(&[u8]);
}

impl Operand for PathBuf {
    fn for_byte_line<F>(&self, mut for_each_line: F) -> Result<()> where F: FnMut(&[u8]) {
        let path_display = format!("{}", self.display());
        let f = File::open(self).with_context(|| format!("Can't open file: {path_display}"))?;
        let reader = reader_for(f);
        reader
            .for_byte_line(|line| {
                for_each_line(line);
                Ok(true)
            })
            .with_context(|| format!("Error reading file: {path_display}"))?;
        Ok(())
    }
}

/// The reader for a second or subsequent operand is a buffered reader with the
/// ability to decode UTF-16 files. I think this results in double-buffering,
/// with one buffer within the `DecodeReaderBytes` value, and another in the
/// `BufReader` that wraps it. I don't know how to work around that.
fn reader_for(file: File) -> BufReader<DecodeReaderBytes<File, Vec<u8>>> {
    BufReader::new(
        DecodeReaderBytesBuilder::new()
            .bom_sniffing(true) // Look at the BOM to detect UTF-16 files and convert to UTF-8
            .strip_bom(true) // Remove the BOM before sending data to us
            .utf8_passthru(true) // Don't enforce UTF-8 (BOM or no BOM)
            .build(file),
    )
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

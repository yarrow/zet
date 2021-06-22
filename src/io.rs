//! Input/Output structs and functions
use std::{
    fs, io,
    io::Read,
    path::{Path, PathBuf},
};

/// Returns `io::stdout`, locked
#[must_use]
pub fn stdout() -> io::Stdout {
    let stdout_for_locking = io::stdout();
    stdout_for_locking.lock();
    stdout_for_locking
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
///     
pub struct ContentsIter {
    files: std::vec::IntoIter<PathBuf>,
}
impl From<Vec<PathBuf>> for ContentsIter {
    /// foo bar baz
    fn from(files: Vec<PathBuf>) -> Self {
        ContentsIter { files: files.into_iter() }
    }
}
impl Iterator for ContentsIter {
    type Item = Result<Vec<u8>, failure::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let path = self.files.next()?;
        Some(match read_and_eol_terminate(&path) {
            Ok(contents) => Ok(contents),
            Err(io_err) => {
                let path = path.to_string_lossy();
                Err(format_err!("Can't read file `{}`: {}", path, io_err))
            }
        })
    }
}

use memchr::memchr;

/// The following functions are based on `std::fs::read` — we can't use
/// `fs::read` directly, because we want to allocate *two* extra bytes
/// (to add `\r\n` if need be), and `fs::read` only allocates one.
pub fn read_and_eol_terminate<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, std::io::Error> {
    fn initial_buffer_size(file: &fs::File) -> usize {
        // Allocate ~~one extra byte~~ two extra bytes so the buffer doesn't
        // need to grow before the final `read` call at the end of the file.
        // Don't worry about `usize` overflow because reading will fail
        // regardless in that case.
        file.metadata().map(|m| m.len() as usize + 2).unwrap_or(0)
    }
    fn inner(path: &Path) -> io::Result<Vec<u8>> {
        let mut file = std::fs::File::open(path)?;
        let mut bytes: Vec<u8> = Vec::with_capacity(initial_buffer_size(&file));
        file.read_to_end(&mut bytes)?;
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
    inner(path.as_ref())
}

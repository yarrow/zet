//! Input/Output structs and functions
use std::{io, fs, path::PathBuf};

use failure;

/// Returns `io::stdout`, locked
#[must_use]
pub fn stdout() -> io::Stdout {
    let stdout_for_locking = io::stdout();
    stdout_for_locking.lock();
    stdout_for_locking
}

/// Given a list of file paths (as a vector of `PathBuf`s), iterates over their contents.
///
/// If `files` is a `Vec<PathBuf>`, then `ContentsIter::from(files)` returns an iterator over
/// the *contents* of the given `files`.  If we want to print out the (entire!) contents of
/// a file if it contains a `b'Z'` anywhere, we could use
///
/// ```no_run
/// # fn main() -> Result<(), failure::Error> {
/// use std::{io::stdout, io::Write, path::PathBuf};
/// use zet::sio::ContentsIter; 
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
        ContentsIter {
            files: files.into_iter(),
        }
    }
}
impl Iterator for ContentsIter {
    type Item = Result<Vec<u8>, failure::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        let path = self.files.next()?;
        Some(match fs::read(&path) {
            Ok(contents) => Ok(contents),
            Err(io_err) => {
                let path = path.to_string_lossy();
                Err(format_err!("Can't read file `{}`: {}", path, io_err))
            }
        })
    }
}

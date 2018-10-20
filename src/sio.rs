use std::fs;
use std::path::PathBuf;

use failure::Error;

pub struct ContentsIter {
    files: std::vec::IntoIter<PathBuf>,
}
impl From<Vec<PathBuf>> for ContentsIter {
    fn from(files: Vec<PathBuf>) -> Self {
        ContentsIter{files: files.into_iter() }
    }
}
impl Iterator for ContentsIter {
    type Item = Result<Vec<u8>, Error>;
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


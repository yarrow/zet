#![cfg_attr(debug_assertions, allow(dead_code, unused))]

extern crate memchr;
use memchr::Memchr;

extern crate failure;
use failure::ResultExt;

use std::path::{Path};
use std::io::{BufReader, Read};
use std::fs::File;

pub fn read_bytes<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, failure::Error> {
    let path = path.as_ref();

    let file = File::open(path).with_context(|_| format!("Could not open file {:?}", path))?;
    let mut file = BufReader::new(file);

    let mut result = Vec::<u8>::new();
    file.read_to_end(&mut result)
        .with_context(|_| format!("Could not read file {:?}", path))?;

    Ok(result)
}

extern crate indexmap;
use indexmap::IndexSet;

type LineSet<'a> = IndexSet<&'a [u8]>;

pub fn lines_of(line_sequence: &[u8]) -> LineSet {
    let mut lines = LineSet::new();
    let mut start = 0;
    for end in Memchr::new(b'\n', line_sequence) {
        lines.insert(&line_sequence[start..end+1]);
        start = end+1;
    }
    if start < line_sequence.len() {
        lines.insert(&line_sequence[start..]);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_order_but_no_duplicates() {
        let x = "x\n".as_bytes();
        let y = "y\n".as_bytes();
        let z = "z\n".as_bytes();
        let with_dups = [x, y, y, z, y, z].concat(); 
        let line_set = lines_of(&with_dups);
        let lines: Vec<&[u8]> = line_set.into_iter().collect();
        assert_eq!(lines, vec![x, y, z]);
    }
}


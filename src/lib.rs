#![cfg_attr(debug_assertions, allow(dead_code, unused))]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy", warn(clippy_pedantic))]

use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;

use failure::Error;
use indexmap::IndexSet;
use memchr::Memchr;

#[macro_use]
extern crate rental;

pub type SetOpResult = Result<(), Error>;

pub mod args;
use crate::args::OpName;

pub(crate) type SliceSet<'a> = IndexSet<&'a [u8]>;
rental! {
    pub mod slice_set {
        use crate::SliceSet;
        #[rental(covariant)]
        pub(crate) struct RentSet {
            lines: Vec<u8>,
            set: SliceSet<'lines>
        }
    }
}
use crate::slice_set::RentSet;

fn rent_set(f: &PathBuf) -> Result<RentSet, Error> {
    let lines = fs::read(f)?;
    Ok(RentSet::new(lines, |x| slice_set(x)))
}

fn is_present_in(x: &[u8], other: &SliceSet<'_>) -> bool {
    other.contains(x)
}
fn is_absent_from(x: &[u8], other: &SliceSet<'_>) -> bool {
    !other.contains(x)
}

pub fn calculate(op: OpName, files: &[PathBuf]) -> SetOpResult {
    if files.is_empty() {
        return Ok(());
    }
    let wanted = match op {
        OpName::Intersect => is_present_in,
        OpName::Diff => is_absent_from,
    };
    let mut result = rent_set(&files[0])?;
    for f in files[1..].iter() {
        let other = rent_set(f)?;
        result.rent_mut(|res| res.retain(|x| wanted(x, other.suffix())));
    }
    for line in result.suffix().iter() {
        stdout().write_all(line)?;
    }
    Ok(())
}

pub(crate) fn slice_set(line_sequence: &[u8]) -> SliceSet<'_> {
    let mut set = SliceSet::new();
    let mut begin = 0;
    for end in Memchr::new(b'\n', line_sequence) {
        set.insert(&line_sequence[begin..=end]);
        begin = end + 1;
    }
    if begin < line_sequence.len() {
        set.insert(&line_sequence[begin..]);
    }
    set
}

/*

// impl Iterator<Item = io::Result<Vec<u8>>>

pub fn intersect(files: &[PathBuf]) -> SetOpResult {
    let contents = fs::read(&files[0])?;
    let mut lines = VecSet::new(&contents);
    for f in files[1..].iter() {
        VecSet::process_file(&mut lines, &VecSet::intersect, &fs::read(f)?);
    }
    for l in lines.iter() {
        stdout().write_all(l)?;
    }
    Ok(())
}

pub fn diff(files: &[PathBuf]) -> SetOpResult {
    let contents = fs::read(&files[0])?;
    let mut result = DiffSet(slice_set(&contents));
    for f in files[1..].iter() {
        result.diminish(&slice_set(&fs::read(f)?));
    }
    for line in result.iter() {
        stdout().write_all(line)?;
    }
    Ok(())
}


trait SubtractiveSet<'a> {
    fn start_with(init: SliceSet<'a>) -> Self;
    fn diminish(&mut self, other: &SliceSet);
    fn iter(&self) -> indexmap::set::Iter<&[u8]>;
}

struct DiffSet<'a>(SliceSet<'a>);
impl<'a> SubtractiveSet<'a> for DiffSet<'a> {
    fn start_with(init: SliceSet<'a>) -> Self {
        DiffSet(init)
    }
    fn diminish(&mut self, other: &SliceSet) {
        self.0.retain(|x| ! other.contains(&x[..]));
    }
    fn iter(&self) -> indexmap::set::Iter<&[u8]> {
        self.0.iter()
    }
}


*/

pub struct VecSet(IndexSet<Vec<u8>>);
impl VecSet {
    pub fn new(line_sequence: &[u8]) -> Self {
        let base = slice_set(line_sequence);
        let mut set = IndexSet::<Vec<u8>>::with_capacity(base.len());
        for line in base.iter() {
            set.insert(line.to_vec());
        }
        VecSet(set)
    }

    pub fn process_file<F>(&mut self, op: F, line_sequence: &[u8])
    where
        F: Fn(&mut Self, &SliceSet<'_>),
    {
        let other = slice_set(line_sequence);
        op(self, &other);
    }
    pub fn intersect(&mut self, other: &SliceSet<'_>) {
        self.0.retain(|x| other.contains(&x[..]));
    }

    pub fn diff(&mut self, line_sequence: &[u8]) {
        let other = slice_set(line_sequence);
        self.0.retain(|x| !other.contains(&x[..]));
    }

    pub fn iter(&self) -> indexmap::set::Iter<'_, Vec<u8>> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xyzq() -> (&'static [u8], &'static [u8], &'static [u8], &'static [u8]) {
        (b"x\n", b"y\n", b"z\n", b"q\n")
    }

    fn sequence<'lines>(set: &VecSet) -> Vec<Vec<u8>> {
        set.iter().cloned().collect()
    }

    #[test]
    fn a_single_file_gives_its_lines_in_the_original_order_without_duplicates() {
        let (x, y, z, _) = xyzq();

        let with_dups = [y, y, x, z, y, z].concat();
        let set = VecSet::new(&with_dups);
        assert_eq!(sequence(&set), vec![y, x, z]);
    }

    #[test]
    fn intersect_returns_lines_in_both_files_ordered_as_in_first_file() {
        let (x, y, z, q) = xyzq();

        let first = [y, x, y, z, y, z].concat();
        let second = [y, x, y, q, q].concat();
        let third = [z, z, q, q, q].concat();

        let mut set = VecSet::new(&first);
        assert_eq!(sequence(&set), vec![y, x, z]);

        set.intersect(&slice_set(&second));
        assert_eq!(sequence(&set), vec![y, x]);

        set.intersect(&slice_set(&third));
        assert_eq!(sequence(&set), Vec::<&[u8]>::new());
    }

    #[test]
    fn diff_returns_lines_in_first_file_but_no_other_ordered_as_in_first_file() {
        let (x, y, z, q) = xyzq();

        let first = [y, x, y, z, y, z].concat();
        let second = [x, q, q].concat();
        let third = [q, q, q, y].concat();

        let mut set = VecSet::new(&first);
        assert_eq!(sequence(&set), vec![y, x, z]);

        set.diff(&second);
        assert_eq!(sequence(&set), vec![y, z]);

        set.diff(&third);
        assert_eq!(sequence(&set), vec![z]);
    }

}

#![cfg_attr(debug_assertions, allow(dead_code, unused))]

extern crate memchr;
use memchr::Memchr;

extern crate indexmap;
use indexmap::IndexSet;

type LineSet<'a> = IndexSet<&'a [u8]>;

struct ShrinkSet<'a> {
    set: LineSet<'a>,
}

fn lines_in_set<'a>(lines: &'a Vec<&'a [u8]>, set: &'a LineSet<'a>)
    -> impl std::iter::Iterator<Item=&'a[u8]> {
    lines.iter().cloned().filter(move |x| set.contains(x))
}

fn line_set<'lines>(line_sequence: &'lines [u8]) -> LineSet<'lines> {
    let mut set = LineSet::new();
    let mut start = 0;
    for end in Memchr::new(b'\n', line_sequence) {
        set.insert(&line_sequence[start..end+1]);
        start = end+1;
    }
    if start < line_sequence.len() {
        set.insert(&line_sequence[start..]);
    }
    set
}

impl<'lines> ShrinkSet<'lines> {
    pub fn new(line_sequence: &'lines [u8]) -> Self  {
        ShrinkSet{set: line_set(line_sequence)}
    }

    pub fn intersect(&mut self, line_sequence: &'lines [u8]) {
        let other = line_set(line_sequence);
        self.set.retain(|&x| other.contains(x));
    }

    pub fn remaining_lines(&self) -> impl std::iter::Iterator<Item=&[u8]> {
        self.set.iter().cloned()
    }

}

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

    fn remaining<'lines>(set: &'lines ShrinkSet<'lines>) -> Vec<&'lines [u8]> {
        set.remaining_lines().collect()
    }

    #[test]
    fn a_single_file_gives_its_lines_in_the_original_order_without_duplicates() {
        let x = "x\n".as_bytes();
        let y = "y\n".as_bytes();
        let z = "z\n".as_bytes();
        let with_dups = [y, y, x, z, y, z].concat(); 
        let set = ShrinkSet::new(&with_dups);
        assert_eq!(remaining(&set), vec![y, x, z]);
    }

    #[test]
    fn result_is_intersection_of_line_sets_in_order_of_first_file() {
        let x = "x\n".as_bytes();
        let y = "y\n".as_bytes();
        let z = "z\n".as_bytes();
        let q = "q\n".as_bytes();
        let first = [y, x, y, z, y, z].concat(); 
        let second = [y, x, y, q, q].concat(); 
        let third = [z, z, q, q, q].concat(); 
        let mut set = ShrinkSet::new(&first);
        assert_eq!(remaining(&set), vec![y, x, z]);
        set.intersect(&second);
        assert_eq!(remaining(&set), vec![y, x]);
        set.intersect(&third);
        assert_eq!(remaining(&set), Vec::<&[u8]>::new());
    }

}


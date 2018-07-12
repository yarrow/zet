#![cfg_attr(debug_assertions, allow(dead_code, unused))]

extern crate memchr;
use memchr::Memchr;

extern crate indexmap;
use indexmap::IndexSet;

type BaseSet<'a> = IndexSet<&'a [u8]>;
pub struct LineSet<'lines>(BaseSet<'lines>);

fn base_set<'lines>(line_sequence: &'lines [u8]) -> BaseSet<'lines> {
    let mut set = BaseSet::new();
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

impl<'lines> LineSet<'lines> {
    pub fn new(line_sequence: &'lines [u8]) -> Self  {
        LineSet(base_set(line_sequence))
    }

    pub fn intersect(&mut self, line_sequence: &[u8]) {
        let other = base_set(line_sequence);
        self.0.retain(|&x| other.contains(x));
    }

    pub fn diff(&mut self, line_sequence: &[u8]) {
        let other = base_set(line_sequence);
        self.0.retain(|&x| ! other.contains(x));
    }

    pub fn iter(&self) -> indexmap::set::Iter<&[u8]> {
        self.0.iter()
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn xyzq() -> (&'static [u8], &'static [u8], &'static [u8], &'static [u8]) {
        (b"x\n", b"y\n", b"z\n", b"q\n")
    }

    fn sequence<'lines>(set: &'lines LineSet<'lines>) -> Vec<&'lines [u8]> {
        set.iter().cloned().collect()
    }

    #[test]
    fn a_single_file_gives_its_lines_in_the_original_order_without_duplicates() {
        let (x, y, z, _) = xyzq();

        let with_dups = [y, y, x, z, y, z].concat();
        let set = LineSet::new(&with_dups);
        assert_eq!(sequence(&set), vec![y, x, z]);
    }

    #[test]
    fn intersect_returns_lines_in_both_files_ordered_as_in_first_file() {
        let (x, y, z, q) = xyzq();

        let first = [y, x, y, z, y, z].concat();
        let second = [y, x, y, q, q].concat();
        let third = [z, z, q, q, q].concat();

        let mut set = LineSet::new(&first);
        assert_eq!(sequence(&set), vec![y, x, z]);

        set.intersect(&second);
        assert_eq!(sequence(&set), vec![y, x]);

        set.intersect(&third);
        assert_eq!(sequence(&set), Vec::<&[u8]>::new());
    }

    #[test]
    fn diff_returns_lines_in_first_file_but_no_other_ordered_as_in_first_file() {
        let (x, y, z, q) = xyzq();

        let first = [y, x, y, z, y, z].concat();
        let second = [x, q, q].concat();
        let third = [q, q, q, y].concat();

        let mut set = LineSet::new(&first);
        assert_eq!(sequence(&set), vec![y, x, z]);

        set.diff(&second);
        assert_eq!(sequence(&set), vec![y, z]);

        set.diff(&third);
        assert_eq!(sequence(&set), vec![z]);
    }

}


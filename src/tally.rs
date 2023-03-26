use anyhow::Result;
use std::fmt::Debug;
pub(crate) trait Select: Copy + PartialEq + Debug {
    fn first_file() -> Self;
    fn next_file(&mut self);
    fn file_number(self) -> u32;
    fn new(file_number: u32) -> Self;
    fn fresh(&self, file_number: u32) -> Self {
        Self::new(file_number)
    }
    fn value(self) -> u32;
    fn modify(&mut self, file_number: u32);
}
pub(crate) trait Bookkeeping: Select {
    fn count(self) -> u32 {
        self.value()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()>;
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct LineCount(u32);
impl Select for LineCount {
    fn first_file() -> Self {
        Self::new(0)
    }
    fn next_file(&mut self) {}
    fn file_number(self) -> u32 {
        0
    }
    fn new(_file_number: u32) -> Self {
        LineCount(1)
    }
    fn value(self) -> u32 {
        self.0
    }
    fn modify(&mut self, _file_number: u32) {
        self.0 += 1
    }
}
impl Bookkeeping for LineCount {
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        write!(out, "{:width$} ", self.0)?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FileCount {
    file_number: u32,
    files_seen: u32,
}
impl Select for FileCount {
    fn first_file() -> Self {
        Self::new(0)
    }
    fn next_file(&mut self) {
        self.file_number += 1;
    }
    fn file_number(self) -> u32 {
        self.file_number
    }
    fn new(file_number: u32) -> Self {
        FileCount { file_number, files_seen: 1 }
    }
    fn value(self) -> u32 {
        self.files_seen
    }
    fn modify(&mut self, file_number: u32) {
        if file_number != self.file_number {
            self.files_seen += 1;
            self.file_number = file_number;
        }
    }
}
impl Bookkeeping for FileCount {
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        write!(out, "{:width$} ", self.files_seen)?;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Noop();
impl Select for Noop {
    fn first_file() -> Self {
        Self::new(0)
    }
    fn next_file(&mut self) {}
    fn file_number(self) -> u32 {
        0
    }
    fn new(_file_number: u32) -> Self {
        Noop()
    }
    fn value(self) -> u32 {
        0
    }
    fn modify(&mut self, _file_number: u32) {}
}
impl Bookkeeping for Noop {
    fn write_count(&self, _width: usize, _out: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct LastFileSeen(u32);
impl Select for LastFileSeen {
    fn first_file() -> Self {
        Self::new(0)
    }
    fn next_file(&mut self) {
        self.0 += 1;
    }
    fn file_number(self) -> u32 {
        self.0
    }
    fn new(file_number: u32) -> Self {
        LastFileSeen(file_number)
    }
    fn value(self) -> u32 {
        self.0
    }
    fn modify(&mut self, file_number: u32) {
        self.0 = file_number;
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Dual<S: Select, B: Bookkeeping> {
    pub(crate) select: S,
    pub(crate) log: B,
}

impl<S: Select, B: Bookkeeping> Select for Dual<S, B> {
    fn first_file() -> Self {
        Self::new(0)
    }
    fn next_file(&mut self) {
        self.select.next_file();
        self.log.next_file();
    }
    fn file_number(self) -> u32 {
        self.select.file_number().max(self.log.file_number())
    }
    fn new(file_number: u32) -> Self {
        Dual { select: S::new(file_number), log: B::new(file_number) }
    }
    fn value(self) -> u32 {
        self.select.value()
    }
    fn modify(&mut self, file_number: u32) {
        self.select.modify(file_number);
        self.log.modify(file_number);
    }
}

impl<S: Select, B: Bookkeeping> Bookkeeping for Dual<S, B> {
    fn count(self) -> u32 {
        self.log.count()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        self.log.write_count(width, out)
    }
}

#[cfg(test)]
mod tally_test {
    use std::fs::File;

    use super::*;
    fn first_file_number<S: Select>() -> u32 {
        S::first_file().file_number()
    }
    #[test]
    fn first_file_file_number_is_zero() {
        assert_eq!(first_file_number::<LineCount>(), 0);
        assert_eq!(first_file_number::<FileCount>(), 0);
        assert_eq!(first_file_number::<Noop>(), 0);
        assert_eq!(first_file_number::<LastFileSeen>(), 0);
        assert_eq!(first_file_number::<Dual<LineCount, LineCount>>(), 0);
        assert_eq!(first_file_number::<Dual<LineCount, FileCount>>(), 0);
        assert_eq!(first_file_number::<Dual<LineCount, Noop>>(), 0);
        assert_eq!(first_file_number::<Dual<FileCount, LineCount>>(), 0);
        assert_eq!(first_file_number::<Dual<FileCount, FileCount>>(), 0);
        assert_eq!(first_file_number::<Dual<FileCount, Noop>>(), 0);
        assert_eq!(first_file_number::<Dual<Noop, LineCount>>(), 0);
        assert_eq!(first_file_number::<Dual<Noop, FileCount>>(), 0);
        assert_eq!(first_file_number::<Dual<Noop, Noop>>(), 0);
        assert_eq!(first_file_number::<Dual<LastFileSeen, LineCount>>(), 0);
        assert_eq!(first_file_number::<Dual<LastFileSeen, FileCount>>(), 0);
        assert_eq!(first_file_number::<Dual<LastFileSeen, Noop>>(), 0);
    }
    fn bump_twice<S: Select>() -> u32 {
        let mut select = S::first_file();
        select.next_file();
        select.next_file();
        select.file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn next_file_increments_file_number_only_for_LastFileSeen_and_FileCount() {
        assert_eq!(bump_twice::<LineCount>(), 0);
        assert_eq!(bump_twice::<FileCount>(), 2);
        assert_eq!(bump_twice::<Noop>(), 0);
        assert_eq!(bump_twice::<LastFileSeen>(), 2);
        assert_eq!(bump_twice::<Dual<LineCount, LineCount>>(), 0);
        assert_eq!(bump_twice::<Dual<LineCount, FileCount>>(), 2);
        assert_eq!(bump_twice::<Dual<LineCount, Noop>>(), 0);
        assert_eq!(bump_twice::<Dual<FileCount, LineCount>>(), 2);
        assert_eq!(bump_twice::<Dual<FileCount, FileCount>>(), 2);
        assert_eq!(bump_twice::<Dual<FileCount, Noop>>(), 2);
        assert_eq!(bump_twice::<Dual<Noop, LineCount>>(), 0);
        assert_eq!(bump_twice::<Dual<Noop, FileCount>>(), 2);
        assert_eq!(bump_twice::<Dual<Noop, Noop>>(), 0);
        assert_eq!(bump_twice::<Dual<LastFileSeen, LineCount>>(), 2);
        assert_eq!(bump_twice::<Dual<LastFileSeen, FileCount>>(), 2);
        assert_eq!(bump_twice::<Dual<LastFileSeen, Noop>>(), 2);
    }
}

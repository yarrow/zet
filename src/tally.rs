use anyhow::Result;
use std::fmt::Debug;
pub(crate) trait Select: Copy + PartialEq + Debug {
    fn new() -> Self;
    fn next_file(&mut self) -> Result<()>;
    fn update_with(&mut self, other: Self);
    fn value(self) -> u32;
}
pub(crate) trait Bookkeeping: Select {
    fn count(self) -> u32 {
        self.value()
    }
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()>;
}

#[cfg(test)]
trait FileNumber: Copy + PartialEq + Debug {
    fn file_number(self) -> Option<u32> {
        None
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct LineCount(u32);
impl Select for LineCount {
    fn new() -> Self {
        LineCount(1)
    }
    fn next_file(&mut self) -> Result<()> {
        Ok(())
    }
    fn update_with(&mut self, _other: Self) {
        self.0 += 1
    }
    fn value(self) -> u32 {
        self.0
    }
}
impl Bookkeeping for LineCount {
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        write!(out, "{:width$} ", self.0)?;
        Ok(())
    }
}
#[cfg(test)]
impl FileNumber for LineCount {}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FileCount {
    file_number: u32,
    files_seen: u32,
}
impl Select for FileCount {
    fn new() -> Self {
        FileCount { file_number: 0, files_seen: 1 }
    }
    fn next_file(&mut self) -> Result<()> {
        self.file_number += 1;
        Ok(())
    }
    fn update_with(&mut self, other: Self) {
        if other.file_number != self.file_number {
            self.files_seen += 1;
            self.file_number = other.file_number;
        }
    }
    fn value(self) -> u32 {
        self.files_seen
    }
}
impl Bookkeeping for FileCount {
    fn write_count(&self, width: usize, out: &mut impl std::io::Write) -> Result<()> {
        write!(out, "{:width$} ", self.files_seen)?;
        Ok(())
    }
}
#[cfg(test)]
impl FileNumber for FileCount {
    fn file_number(self) -> Option<u32> {
        Some(self.file_number)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Noop();
impl Select for Noop {
    fn new() -> Self {
        Noop()
    }
    fn next_file(&mut self) -> Result<()> {
        Ok(())
    }
    fn update_with(&mut self, _other: Self) {}
    fn value(self) -> u32 {
        0
    }
}
impl Bookkeeping for Noop {
    fn write_count(&self, _width: usize, _out: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}
#[cfg(test)]
impl FileNumber for Noop {}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct LastFileSeen(u32);
impl Select for LastFileSeen {
    fn new() -> Self {
        LastFileSeen(0)
    }
    fn next_file(&mut self) -> Result<()> {
        self.0 += 1;
        Ok(())
    }
    fn update_with(&mut self, other: Self) {
        self.0 = other.0
    }
    fn value(self) -> u32 {
        self.0
    }
}
#[cfg(test)]
impl FileNumber for LastFileSeen {
    fn file_number(self) -> Option<u32> {
        Some(self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Dual<S: Select, B: Bookkeeping> {
    pub(crate) select: S,
    pub(crate) log: B,
}

impl<S: Select, B: Bookkeeping> Select for Dual<S, B> {
    fn new() -> Self {
        Dual { select: S::new(), log: B::new() }
    }
    fn next_file(&mut self) -> Result<()> {
        self.select.next_file()?;
        self.log.next_file()
    }
    fn update_with(&mut self, other: Self) {
        self.select.update_with(other.select);
        self.log.update_with(other.log);
    }
    fn value(self) -> u32 {
        self.select.value()
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
impl<S: Select + FileNumber, B: Bookkeeping + FileNumber> FileNumber for Dual<S, B> {
    fn file_number(self) -> Option<u32> {
        self.select.file_number().or(self.log.file_number())
    }
}

#[cfg(test)]
mod tally_test {
    use std::fs::File;

    use super::*;
    fn new_file_number<S: Select + FileNumber>() -> Option<u32> {
        S::new().file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn first_file_file_number_is_None_for_Noop_and_LineCount_and_Some_0_otherwise() {
        assert_eq!(new_file_number::<LineCount>(), None);
        assert_eq!(new_file_number::<FileCount>(), Some(0));
        assert_eq!(new_file_number::<Noop>(), None);
        assert_eq!(new_file_number::<LastFileSeen>(), Some(0));
        assert_eq!(new_file_number::<Dual<LineCount, LineCount>>(), None);
        assert_eq!(new_file_number::<Dual<LineCount, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<LineCount, Noop>>(), None);
        assert_eq!(new_file_number::<Dual<FileCount, LineCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<FileCount, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<FileCount, Noop>>(), Some(0));
        assert_eq!(new_file_number::<Dual<Noop, LineCount>>(), None);
        assert_eq!(new_file_number::<Dual<Noop, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<Noop, Noop>>(), None);
        assert_eq!(new_file_number::<Dual<LastFileSeen, LineCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<LastFileSeen, FileCount>>(), Some(0));
        assert_eq!(new_file_number::<Dual<LastFileSeen, Noop>>(), Some(0));
    }

    fn bump_twice<S: Select>() -> S {
        let mut select = S::new();
        select.next_file().unwrap();
        select.next_file().unwrap();
        select
    }
    fn bump_twice_file_number<S: Select + FileNumber>() -> Option<u32> {
        bump_twice::<S>().file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn next_file_increments_file_number_only_for_LastFileSeen_and_FileCount() {
        assert_eq!(bump_twice_file_number::<LineCount>(), None);
        assert_eq!(bump_twice_file_number::<FileCount>(), Some(2));
        assert_eq!(bump_twice_file_number::<Noop>(), None);
        assert_eq!(bump_twice_file_number::<LastFileSeen>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LineCount, LineCount>>(), None);
        assert_eq!(bump_twice_file_number::<Dual<LineCount, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LineCount, Noop>>(), None);
        assert_eq!(bump_twice_file_number::<Dual<FileCount, LineCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<FileCount, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<FileCount, Noop>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<Noop, LineCount>>(), None);
        assert_eq!(bump_twice_file_number::<Dual<Noop, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<Noop, Noop>>(), None);
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, LineCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, FileCount>>(), Some(2));
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, Noop>>(), Some(2));
    }

    fn assert_update_with_sets_self_file_number_to_arguments<S: Select + FileNumber>() {
        let mut naive = S::new();
        let mut other = S::new();
        other.next_file().unwrap();
        other.next_file().unwrap();
        naive.update_with(other);
        assert_eq!(naive.file_number(), other.file_number());
    }
    #[test]
    fn update_with_sets_file_number_to_its_arguments_file_number() {
        assert_update_with_sets_self_file_number_to_arguments::<LineCount>();
        assert_update_with_sets_self_file_number_to_arguments::<FileCount>();
        assert_update_with_sets_self_file_number_to_arguments::<Noop>();
        assert_update_with_sets_self_file_number_to_arguments::<LastFileSeen>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<LineCount, LineCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<LineCount, FileCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<LineCount, Noop>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<FileCount, LineCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<FileCount, FileCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<FileCount, Noop>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<Noop, LineCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<Noop, FileCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<Noop, Noop>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<LastFileSeen, LineCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<LastFileSeen, FileCount>>();
        assert_update_with_sets_self_file_number_to_arguments::<Dual<LastFileSeen, Noop>>();
    }
}

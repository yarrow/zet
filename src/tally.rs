use anyhow::Result;
use std::fmt::Debug;
pub(crate) trait Select: Copy + PartialEq + Debug {
    fn new() -> Self;
    fn next_file(&mut self);
    fn update_with(&mut self, _the_vogue: Self);
    fn file_number(self) -> u32;
    fn value(self) -> u32;
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
    fn new() -> Self {
        LineCount(1)
    }
    fn next_file(&mut self) {}
    fn update_with(&mut self, _the_vogue: Self) {
        self.0 += 1
    }
    fn file_number(self) -> u32 {
        0
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FileCount {
    file_number: u32,
    files_seen: u32,
}
impl Select for FileCount {
    fn new() -> Self {
        FileCount { file_number: 1, files_seen: 1 }
    }
    fn next_file(&mut self) {
        self.file_number += 1;
    }
    fn update_with(&mut self, the_vogue: Self) {
        if the_vogue.file_number != self.file_number {
            self.files_seen += 1;
            self.file_number = the_vogue.file_number;
        }
    }
    fn file_number(self) -> u32 {
        self.file_number
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Noop();
impl Select for Noop {
    fn new() -> Self {
        Noop()
    }
    fn next_file(&mut self) {}
    fn update_with(&mut self, _the_vogue: Self) {}
    fn file_number(self) -> u32 {
        0
    }
    fn value(self) -> u32 {
        0
    }
}
impl Bookkeeping for Noop {
    fn write_count(&self, _width: usize, _out: &mut impl std::io::Write) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct LastFileSeen(u32);
impl Select for LastFileSeen {
    fn new() -> Self {
        LastFileSeen(1)
    }
    fn next_file(&mut self) {
        self.0 += 1;
    }
    fn file_number(self) -> u32 {
        self.0
    }
    fn update_with(&mut self, the_vogue: Self) {
        self.0 = the_vogue.0
    }
    fn value(self) -> u32 {
        self.0
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
    fn next_file(&mut self) {
        self.select.next_file();
        self.log.next_file();
    }
    fn update_with(&mut self, the_vogue: Self) {
        self.select.update_with(the_vogue.select);
        self.log.update_with(the_vogue.log);
    }
    fn file_number(self) -> u32 {
        self.select.file_number().max(self.log.file_number())
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
mod tally_test {
    use std::fs::File;

    use super::*;
    fn first_file_number<S: Select>() -> u32 {
        S::new().file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn first_file_file_number_is_zero_for_Noop_and_LineCount_one_otherwise() {
        assert_eq!(first_file_number::<LineCount>(), 0);
        assert_eq!(first_file_number::<FileCount>(), 1);
        assert_eq!(first_file_number::<Noop>(), 0);
        assert_eq!(first_file_number::<LastFileSeen>(), 1);
        assert_eq!(first_file_number::<Dual<LineCount, LineCount>>(), 0);
        assert_eq!(first_file_number::<Dual<LineCount, FileCount>>(), 1);
        assert_eq!(first_file_number::<Dual<LineCount, Noop>>(), 0);
        assert_eq!(first_file_number::<Dual<FileCount, LineCount>>(), 1);
        assert_eq!(first_file_number::<Dual<FileCount, FileCount>>(), 1);
        assert_eq!(first_file_number::<Dual<FileCount, Noop>>(), 1);
        assert_eq!(first_file_number::<Dual<Noop, LineCount>>(), 0);
        assert_eq!(first_file_number::<Dual<Noop, FileCount>>(), 1);
        assert_eq!(first_file_number::<Dual<Noop, Noop>>(), 0);
        assert_eq!(first_file_number::<Dual<LastFileSeen, LineCount>>(), 1);
        assert_eq!(first_file_number::<Dual<LastFileSeen, FileCount>>(), 1);
        assert_eq!(first_file_number::<Dual<LastFileSeen, Noop>>(), 1);
    }

    fn bump_twice<S: Select>() -> S {
        let mut select = S::new();
        select.next_file();
        select.next_file();
        select
    }
    fn bump_twice_file_number<S: Select>() -> u32 {
        bump_twice::<S>().file_number()
    }
    #[test]
    #[allow(non_snake_case)]
    fn next_file_increments_file_number_only_for_LastFileSeen_and_FileCount() {
        assert_eq!(bump_twice_file_number::<LineCount>(), 0);
        assert_eq!(bump_twice_file_number::<FileCount>(), 3);
        assert_eq!(bump_twice_file_number::<Noop>(), 0);
        assert_eq!(bump_twice_file_number::<LastFileSeen>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<LineCount, LineCount>>(), 0);
        assert_eq!(bump_twice_file_number::<Dual<LineCount, FileCount>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<LineCount, Noop>>(), 0);
        assert_eq!(bump_twice_file_number::<Dual<FileCount, LineCount>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<FileCount, FileCount>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<FileCount, Noop>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<Noop, LineCount>>(), 0);
        assert_eq!(bump_twice_file_number::<Dual<Noop, FileCount>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<Noop, Noop>>(), 0);
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, LineCount>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, FileCount>>(), 3);
        assert_eq!(bump_twice_file_number::<Dual<LastFileSeen, Noop>>(), 3);
    }

    fn assert_update_with_sets_self_file_number_to_arguments<S: Select>() {
        let mut naive = S::new();
        let mut the_vogue = S::new();
        the_vogue.next_file();
        the_vogue.next_file();
        naive.update_with(the_vogue);
        assert_eq!(naive.file_number(), the_vogue.file_number());
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

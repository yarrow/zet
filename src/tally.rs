use anyhow::Result;
use std::fmt::Debug;
pub(crate) trait Select: Copy + PartialEq + Debug {
    fn first_file() -> Self;
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

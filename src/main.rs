#![cfg_attr(debug_assertions, allow(dead_code, unused))]

extern crate setop;
use setop::LineSet;

#[macro_use]
extern crate quicli;

use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::result;
use std::fs;

use quicli::prelude::*;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "setop", about = "find the union or intersection of files considered as sets of lines",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
    after_help=
"Each line is output at most once, no matter how many times it occurs in the file(s). Lines are not sorted, but are printed in the order they occur in the input."
)]
struct Args {
    #[structopt(
        name="intersect|union|diff|once",
        raw(next_line_help = "true"), long_help=
"Each operation prints lines meeting a different condition:
    Operation  Prints lines appearing in
    ========== =========================
    intersect: EVERY file
    union:     ANY file
    diff:      the FIRST file, and no other
    once:      exactly ONE file"
    )]
    op: OpName,
    #[structopt(
        parse(from_os_str),
        help = "Input files", raw(next_line_help = "true"),
    )]
    file: Vec<PathBuf>,
}

#[derive(Debug)]
enum OpName {
    Intersect,
    Diff,
}
impl FromStr for OpName {
    type Err = String;
    fn from_str(s: &str) -> result::Result<Self, <Self as FromStr>::Err> {
        match &*s.to_ascii_lowercase() {
            "intersect" => Ok(OpName::Intersect),
            "diff" => Ok(OpName::Diff),
            _ => Err("Expected intersect, diff, ...".to_owned()),
        }
    }
}

main!(|args: Args| {
    let files = args.file;
    if files.is_empty() { return Ok(()) }

    let contents = fs::read(&files[0])?;
    let mut lines = LineSet::new(&contents);
    let rest = files[1..].iter();

    use self::OpName::*;
    match args.op {
        Intersect => for f in rest {
            lines.intersect(&fs::read(f)?);
        }
        Diff => for f in rest {
            lines.diff(&fs::read(f)?);
        }
    }

    for l in lines.iter() {
        io::stdout().write(l)?;
    }
});

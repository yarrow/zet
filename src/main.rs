#![cfg_attr(debug_assertions, allow(dead_code, unused))]

use std::fs;
use std::io::{stdout, Write};

extern crate structopt;
use structopt::StructOpt;
extern crate failure;
use failure::Error;

extern crate setop;
use setop::LineSet;
use setop::args::{OpName, Args, parse_args};

fn main() -> Result<(), Error> {
    let args = parse_args();
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
        stdout().write(l)?;
    }
    Ok(())
}

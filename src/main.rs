#![cfg_attr(debug_assertions, allow(dead_code, unused))]

#[macro_use]
extern crate quicli;

use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::result;

use quicli::prelude::*;
use quicli::prelude::structopt::clap;

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
}
impl FromStr for OpName {
    type Err = String;
    fn from_str(s: &str) -> result::Result<Self, <Self as FromStr>::Err> {
        match &*s.to_ascii_lowercase() {
            "intersect" => Ok(OpName::Intersect),
            _ => Err("Expected intersect, ...".to_owned()),
        }
    }
}

use std::path::{Path};
use std::io::{BufReader, BufWriter, Read};
use std::fs::File;
fn reed_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>> {
    let path = path.as_ref();
    use std::fs;
//    println!("{:?}", fs::metadata(path)?.file_type());
//    ensure!(
//        path.exists() && path.is_file(),
//        "Path {:?} is not a file!",
//        path
//    );

    let file = File::open(path).with_context(|_| format!("Could not open file {:?}", path))?;
    let mut file = BufReader::new(file);

    let mut result = Vec::<u8>::new();
    file.read_to_end(&mut result)
        .with_context(|_| format!("Could not read file {:?}", path))?;

    Ok(result)
}

main!(|args: Args| {
    let files = args.file;
    if files.len() == 0 { return Ok(()) }
    for f in files {
        let contents = reed_file(f)?;
        io::stdout().write(&contents);
    }
});

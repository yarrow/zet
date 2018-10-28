//! Code to parse the command line using `structop` and `clap`, and definitions
//! of the parsed result

use std::path::PathBuf;
use std::result;
use std::str::FromStr;

use structopt::StructOpt;

/// Returns the parsed command line: the `Args` return value's `op` field is the set operation
/// desired, and the `files` field holds the files to take as operands.
pub fn parsed() -> Args {
    Args::from_args()
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "setop",
    about = "Calcuate the union, intersection, and so forth of files considered as sets of lines",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp"),
    after_help = "Each line is output at most once, no matter how many times it occurs in the file(s). Lines are not sorted, but are printed in the order they occur in the input."
)]
/// `Args` contains the parsed command line.
pub struct Args {
    #[structopt(
        name = "intersect|union|diff|single|multiple",
        raw(next_line_help = "true"),
        long_help = "Each operation prints lines meeting a different condition:
    Operation  Prints lines appearing in
    ========== =========================
    intersect: EVERY file
    union:     ANY file
    diff:      the FIRST file, and no other
    single:    exactly ONE file
    multiple:  MORE THAN one file"
    )]
    /// `op` is the set operation requested
    pub op: OpName,
    #[structopt(
        parse(from_os_str),
        help = "Input files",
        raw(next_line_help = "true"),
    )]
    /// `files` is the list of files from the command line
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
/// Name of the requested operation
pub enum OpName {
    /// Print the lines present in every file
    Intersect,
    /// Print the lines present in any file
    Union,
    /// Print the lines present in the first file but no other
    Diff,
    /// Print the lines present in exactly one file
    Single,
    /// Print the lines present in two or more files
    Multiple,
}
impl FromStr for OpName {
    type Err = String;
    fn from_str(s: &str) -> result::Result<Self, <Self as FromStr>::Err> {
        match &*s.to_ascii_lowercase() {
            "intersect" => Ok(OpName::Intersect),
            "union" => Ok(OpName::Union),
            "diff" => Ok(OpName::Diff),
            "single" => Ok(OpName::Single),
            "multiple" => Ok(OpName::Multiple),
            _ => Err("Expected intersect, union, diff, single, or multiple".to_owned()),
        }
    }
}

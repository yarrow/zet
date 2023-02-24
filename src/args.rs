//! Code to parse the command line using `structop` and `clap`, and definitions
//! of the parsed result

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Returns the parsed command line: the `Args` return value's `op` field is the set operation
/// desired, and the `files` field holds the files to take as operands.
#[must_use]
pub fn parsed() -> Args {
    let parsed = CliArgs::parse();
    let op = match parsed.op {
        CliName::Help => unimplemented!(),
        CliName::Intersect => OpName::Intersect,
        CliName::Union => OpName::Union,
        CliName::Diff => OpName::Diff,
        CliName::Single => OpName::Single,
        CliName::Multiple => OpName::Multiple,
    };
    Args { op, files: parsed.files }
}

pub struct Args {
    /// `op` is the set operation requested
    pub op: OpName,
    /// `files` is the list of files from the command line
    pub files: Vec<PathBuf>,
}
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
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

#[derive(Debug, Parser)]
#[command(name = "zet")]
/// `Args` contains the parsed command line.
struct CliArgs {
    #[arg(value_enum)]
    /// `op` is the set operation requested
    op: CliName,
    #[arg(name = "Input files")]
    /// `files` is the list of files from the command line
    files: Vec<PathBuf>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, ValueEnum)]
/// Name of the requested operation
enum CliName {
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
    /// Print a help message
    Help,
}

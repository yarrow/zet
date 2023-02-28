//! Code to parse the command line using `structop` and `clap`, and definitions
//! of the parsed result

use crate::help;
use crate::styles::{self, ColorChoice, StyleSheet};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Returns the parsed command line: the `Args` return value's `op` field is the set operation
/// desired, and the `files` field holds the files to take as operands.
#[must_use]
pub fn parsed() -> Args {
    let parsed = CliArgs::parse();
    if parsed.help {
        help_and_exit();
    }
    if parsed.version {
        version_and_exit();
    }
    let Some(op) = parsed.op else { help_and_exit() };
    let op = match op {
        CliName::Help => help_and_exit(),
        CliName::Intersect => OpName::Intersect,
        CliName::Union => OpName::Union,
        CliName::Diff => OpName::Diff,
        CliName::Single => OpName::Single,
        CliName::Multiple => OpName::Multiple,
    };
    Args { op, files: parsed.files }
}

fn help_and_exit() -> ! {
    exit_after(help::print);
}
fn version_and_exit() -> ! {
    exit_after(help::print_version)
}
fn exit_after(print_something: impl FnOnce(&StyleSheet)) -> ! {
    styles::init();
    print_something(styles::colored(ColorChoice::Auto));
    std::process::exit(0);
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
    #[arg(short, long)]
    /// Like the `help` command, the `-h` or `--help` flags tell us to print the help message
    /// and exit
    help: bool,
    #[arg(short('V'), long)]
    /// The `-V` or `--version` flags tell us to print our name and version, then exit
    version: bool,
    #[arg(value_enum)]
    /// `op` is the set operation requested
    op: Option<CliName>,
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

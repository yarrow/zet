//! Code to parse the command line using `clap`, and definitions of the parsed result

use crate::help;
use crate::styles::{set_color_choice, ColorChoice};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Returns the parsed command line: the `Args` return value's `op` field is the set operation
/// desired, and the `files` field holds the files to take as operands.
#[must_use]
pub fn parsed() -> Args {
    let parsed = CliArgs::parse();
    let cc = parsed.color.unwrap_or(ColorChoice::Auto);
    set_color_choice(cc);
    if parsed.help {
        help_and_exit();
    }
    let Some(op) = parsed.command else { help_and_exit() };
    if op == CliName::Help {
        help_and_exit()
    }
    if parsed.version {
        println!("{}", help::version());
        exit_success();
    }
    if parsed.by_file {
        match op {
            CliName::Single | CliName::Multiple => (),
            _ => {
                eprintln!("{}", help::by_file_usage());
                exit_usage();
            }
        }
    }
    let op = match op {
        CliName::Help => help_and_exit(), // This can't happen, but...
        CliName::Intersect => OpName::Intersect,
        CliName::Union => OpName::Union,
        CliName::Diff => OpName::Diff,
        CliName::Single => {
            if parsed.by_file {
                OpName::SingleByFile
            } else {
                OpName::Single
            }
        }
        CliName::Multiple => {
            if parsed.by_file {
                OpName::MultipleByFile
            } else {
                OpName::Multiple
            }
        }
    };
    Args { op, count_lines: parsed.count, files: parsed.files }
}

fn help_and_exit() -> ! {
    help::print();
    exit_success();
}

const SUCCESS_CODE: i32 = 0;
const USAGE_CODE: i32 = 2; // Because `clap` uses 2
fn exit_success() -> ! {
    safe_exit(SUCCESS_CODE)
}
fn exit_usage() -> ! {
    safe_exit(USAGE_CODE)
}
/// From clap
fn safe_exit(code: i32) -> ! {
    use std::io::Write;

    let _ = std::io::stdout().lock().flush();
    let _ = std::io::stderr().lock().flush();

    std::process::exit(code)
}

pub struct Args {
    /// `op` is the set operation requested
    pub op: OpName,
    /// Should we count the number of times each line occurs?
    pub count_lines: bool,
    /// `files` is the list of files from the command line
    pub files: Vec<PathBuf>,
}

/// Set operation to perform
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum OpName {
    /// Print the lines present in every file
    Intersect,
    /// Print the lines present in any file
    Union,
    /// Print the lines present in the first file but no other
    Diff,
    /// Print the lines present exactly once in the entire input
    Single,
    /// Print the lines present in exactly one file
    SingleByFile,
    /// Print the lines present more than once in the entire input
    Multiple,
    /// Print the lines present in two or more files
    MultipleByFile,
}

#[derive(Debug, Parser)]
#[command(name = "zet")]
/// `CliArgs` contains the parsed command line.
struct CliArgs {
    #[arg(short, long)]
    /// The --count flag tells `zet` to count the number of times a line occurs in the input
    count: bool,

    #[arg(long)]
    /// With `--by-file`, the `single` and `multiple` commands count a line as occuring
    /// once if it's only contained in one file, even if it occurs many times in that file.
    by_file: bool,

    #[arg(short, long)]
    /// Like the `help` command, the `-h` or `--help` flags tell us to print the help message
    /// and exit
    help: bool,

    #[arg(short('V'), long)]
    /// The `-V` or `--version` flags tell us to print our name and version, then exit
    version: bool,

    #[arg(long)]
    /// The `color` flag tells us whether to print color or not (Auto means Yes, if
    /// stdout is a terminal that supports color)
    color: Option<ColorChoice>,

    #[arg(value_enum)]
    /// `op` is the set operation requested
    command: Option<CliName>,

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

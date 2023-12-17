//! Code to parse the command line using `clap`, and definitions of the parsed result

use crate::help;
use crate::operations::LogType;
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
    if parsed.version {
        println!("{}", help::version());
        exit_success();
    }
    let Some(op) = parsed.command else { help_and_exit() };
    if op == CliName::Help {
        help_and_exit()
    }
    let log_type = if parsed.count_files {
        LogType::Files
    } else if parsed.count_lines {
        LogType::Lines
    } else if parsed.count {
        if parsed.files {
            LogType::Files
        } else {
            LogType::Lines
        }
    } else {
        LogType::None
    };

    let op = match op {
        CliName::Help => help_and_exit(), // This can't happen, but...
        CliName::Intersect => OpName::Intersect,
        CliName::Union => OpName::Union,
        CliName::Diff => OpName::Diff,
        CliName::Single => {
            if parsed.files {
                OpName::SingleByFile
            } else {
                OpName::Single
            }
        }
        CliName::Multiple => {
            if parsed.files {
                OpName::MultipleByFile
            } else {
                OpName::Multiple
            }
        }
    };
    Args { op, log_type, paths: parsed.paths }
}

fn help_and_exit() -> ! {
    help::print();
    exit_success();
}

const SUCCESS_CODE: i32 = 0;
fn exit_success() -> ! {
    safe_exit(SUCCESS_CODE)
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
    pub log_type: LogType,
    /// `paths` is the list of files from the command line
    pub paths: Vec<PathBuf>,
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
    #[arg(long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// The --count-files flag tells `zet` to report the number of files a line occurs in
    count_files: bool,

    #[arg(long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// The --count-lines flag tells `zet` to report the times a line appears in the entire input
    count_lines: bool,

    #[arg(long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// The --count-none flag tells `zet` to turn off reporting
    count_none: bool,

    #[arg(short, long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// The --count is like --count-lines, but --files makes it act like --count-files
    count: bool,

    #[arg(long, alias("file"), overrides_with_all(["files", "lines"]))]
    /// With `--files`, the `single` and `multiple` commands count a line as occuring
    /// once if it's only contained in one file, even if it occurs many times in that file.
    files: bool,

    #[arg(long, alias("line"), overrides_with_all(["files", "lines"]))]
    /// `--lines` is the default. Specify it explicitly to override a previous `--files`
    lines: bool,

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
    /// `paths` is the list of file paths from the command line
    paths: Vec<PathBuf>,
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

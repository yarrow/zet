//! Code to parse the command line using `clap`, and definitions of the parsed result

use crate::operations::LogType;
use crate::styles::{ColorChoice, APP_NAME, ITEM, STYLES, TITLE};
use clap::{Command, CommandFactory, Parser, ValueEnum};
use clap_complete::Shell;
use std::fmt::Write as _;
use std::path::PathBuf;

/// Returns the parsed command line: the `Args` return value's `op` field is the set operation
/// desired, and the `files` field holds the files to take as operands.
#[must_use]
pub fn parsed() -> Args {
    let parsed = CliArgs::parse();
    if let Some(shell) = parsed.completions {
        clap_complete::generate(shell, &mut CliArgs::command(), "zet", &mut std::io::stdout());
        exit_success();
    }
    let cc = parsed.color.unwrap_or(ColorChoice::Auto);
    if parsed.help {
        help_and_exit(cc);
    }
    if parsed.version {
        version_and_exit(cc);
    }
    let Some(op) = parsed.command else { help_and_exit(cc) };
    let op = match op {
        CliName::Help => help_and_exit(cc),
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

    Args { op, log_type, paths: parsed.paths }
}

fn help_and_exit(cc: ColorChoice) -> ! {
    exit_after(help_command().color(cc.into()).print_help())
}

fn version_and_exit(cc: ColorChoice) -> ! {
    use std::io::Write;
    let mut stdout = anstream::AutoStream::new(std::io::stdout().lock(), cc.into());
    let version = std::env!("CARGO_PKG_VERSION");
    exit_after(writeln!(stdout, "{APP_NAME}zet{APP_NAME:#} {version}"))
}

fn exit_after(result: std::io::Result<()>) -> ! {
    let code = match result {
        Err(e) => {
            eprintln!("failed printing to stdout: {e}");
            1
        }
        Ok(()) => SUCCESS_CODE,
    };
    safe_exit(code);
}

/// The `Command` we print help from. We parse `zet`'s commands as the values of a positional
/// argument, but we want help to list them the way `clap` lists subcommands. So, for display
/// only, we hide the positional arguments and add each `CliName` as a subcommand, taking its
/// description from the `CliName` doc comments. Also for display only, we show `--files` and
/// `--lines` as `--file[s]` and `--line[s]`, since each can be spelled either way.
fn help_command() -> Command {
    let commands =
        CliName::value_variants().iter().filter_map(CliName::to_possible_value).map(|command| {
            let about = command.get_help().cloned().unwrap_or_default();
            Command::new(command.get_name().to_owned()).about(about)
        });
    CliArgs::command()
        .mut_args(|arg| if arg.is_positional() { arg.hide(true) } else { arg })
        .mut_arg("files", |arg| arg.long("file[s]"))
        .mut_arg("lines", |arg| arg.long("line[s]"))
        .disable_help_subcommand(true)
        .subcommands(commands)
}

fn usage() -> String {
    format!("{APP_NAME}zet{APP_NAME:#} [OPTIONS] <COMMAND> <PATH...>")
}

fn help_template() -> String {
    format!(
        "{APP_NAME}{{name}}{APP_NAME:#} {{version}}\n\
         {{about}}\n\n\
         {{usage-heading}} {{usage}}\n\n\
         {{all-args}}{{after-help}}"
    )
}

fn after_help() -> String {
    const SIMILAR_TO: [(&str, &str); 5] = [
        ("union", "uniq"),
        ("intersect", "comm -12"),
        ("diff", "comm -23"),
        ("single", "uniq -u"),
        ("multiple", "uniq -d"),
    ];
    let mut text = format!(
        "Each line is output at most once, no matter how many times it occurs in the input. \
         Lines are printed in the order they occur in the input.\n\n\
         {TITLE}Similar to:{TITLE:#}"
    );
    for (command, similar) in SIMILAR_TO {
        let _ = write!(text, "\n  {ITEM}{command:<9}{ITEM:#}  {similar}");
    }
    text
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
#[command(
    name = "zet",
    version,
    // We handle `--help` and `--version` ourselves, so they can respect `--color`
    disable_help_flag = true,
    disable_version_flag = true,
    styles = STYLES,
    override_usage = usage(),
    help_template = help_template(),
    after_help = after_help()
)]
/// Calculate the union, intersection, set difference and so forth of files considered as sets of
/// lines
//
// The doc comments here and in `CliName` are the text of `zet`'s help message, and the
// descriptions shown by shell completions.
struct CliArgs {
    #[arg(long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// Show the number of times each line occurs in the input
    count_lines: bool,

    #[arg(long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// Show the number of files each line occurs in
    count_files: bool,

    #[arg(short, long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// Like --count-lines, but if --files is present, like --count-files
    count: bool,

    #[arg(long, overrides_with_all(["count", "count_files", "count_lines", "count_none"]))]
    /// Turn off counting, overriding any earlier count option
    count_none: bool,

    #[arg(long, alias("file"), overrides_with_all(["files", "lines"]))]
    /// To count as multiple, a line must occur in more than one file. Affects the single and
    /// multiple commands, as well as the -c and --count options
    files: bool,

    #[arg(long, alias("line"), overrides_with_all(["files", "lines"]))]
    /// The default: a line that occurs twice in one file counts as multiple. Overrides any
    /// earlier --files
    lines: bool,

    #[arg(long, value_name = "WHEN")]
    /// When to color the help message
    color: Option<ColorChoice>,

    #[arg(long, value_name = "SHELL")]
    /// Print shell completions for the given shell and exit
    completions: Option<Shell>,

    #[arg(short, long)]
    /// Print this message
    help: bool,

    #[arg(short('V'), long)]
    /// Print version
    version: bool,

    #[arg(value_enum)]
    /// The set operation to perform
    command: Option<CliName>,

    #[arg(value_name = "PATH")]
    /// The input files (standard input, if there are none)
    paths: Vec<PathBuf>,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, ValueEnum)]
/// Name of the requested operation
enum CliName {
    /// Prints lines appearing in ANY input file
    Union,
    /// Prints lines appearing in EVERY input file
    Intersect,
    /// Prints lines appearing in the FIRST input file and no other
    Diff,
    /// Prints lines appearing exactly once; with --file, in exactly one file
    Single,
    /// Prints lines appearing more than once; with --files, in more than one file
    Multiple,
    /// Print this message
    Help,
}

use anyhow::{bail, Result};
use is_terminal::IsTerminal;
use std::io;
use zet::args::OpName;
use zet::operands::first_and_rest;
use zet::operations::{calculate, LogType};

fn main() -> Result<()> {
    let args = zet::args::parsed();

    let files = first_and_rest(&args.files).or_else(|| first_and_rest(&["-".into()]));
    let (first_operand, rest, number_of_operands) = match files {
        None => {
            bail!("This can't happen: with no file arguments, zet should read from standard input")
        }
        Some((first, others, others_len)) => (first?, others, others_len + 1),
    };

    let mut op = args.op;
    if number_of_operands == 1 {
        use OpName::*;
        match op {
            // For a single operand, Union is slightly more efficient, and its
            // result is identical to Intersect, Diff, and SingleByFile
            Union | Intersect | Diff | SingleByFile => op = Union, // Union is slightly more efficient
            // No line can occur in multiple files if there is only one file
            MultipleByFile => return Ok(()),
            // Even for a single operand, the results of Single and Multiple
            // differ from that of Union
            Single | Multiple => {}
        }
    }

    let first = first_operand.as_slice();
    let log_type = if args.count_lines { LogType::Lines } else { LogType::None };
    if io::stdout().is_terminal() {
        calculate(op, log_type, first, rest, io::stdout().lock())?;
    } else {
        calculate(op, log_type, first, rest, io::BufWriter::new(io::stdout().lock()))?;
    };
    Ok(())
}

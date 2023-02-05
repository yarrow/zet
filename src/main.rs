use anyhow::Result;
use is_terminal::IsTerminal;
use std::io;
use zet::args::OpName;
use zet::operands::first_and_rest;
use zet::operations::calculate;

fn main() -> Result<()> {
    let args = zet::args::parsed();

    let (first_operand, rest, number_of_operands) = match first_and_rest(&args.files) {
        None => return Ok(()), // No operands implies an empty result
        Some((first, others, others_len)) => (first?, others, others_len + 1),
    };

    let op = if number_of_operands == 1 && args.op == OpName::Multiple {
        // Since there is only one operand, no line can occur in multiple
        // operands, so we return at once with an empty result.
        return Ok(());
    } else if number_of_operands == 1 {
        // For a single operand, all operations except Multiple are equivalent
        // to Union, and Union is slightly more efficient than the others.
        OpName::Union
    } else {
        args.op
    };

    let first = first_operand.as_slice();
    if io::stdout().is_terminal() {
        calculate(op, first, rest, io::stdout().lock())?;
    } else {
        calculate(op, first, rest, io::BufWriter::new(io::stdout().lock()))?;
    };
    Ok(())
}

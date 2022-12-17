use anyhow::Result;
use std::io;
use zet::args::OpName;
use zet::operands::first_and_rest;
use zet::operations::calculate;
fn main() -> Result<()> {
    let args = zet::args::parsed();

    let (first_operand, rest) = match first_and_rest(&args.files) {
        None => return Ok(()), // No operands implies an empty result
        Some((first, others)) => (first?, others),
    };

    let first = first_operand.as_slice();
    if atty::is(atty::Stream::Stdout) {
        calculate(args.op, first, &rest, io::stdout().lock())?;
    } else {
        calculate(args.op, first, &rest, io::BufWriter::new(io::stdout().lock()))?;
    };
    Ok(())
}

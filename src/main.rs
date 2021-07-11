use anyhow::Result;
fn main() -> Result<()> {
    let args = zet::args::parsed();

    // We use `(first, rest, set_writer)` because the `io` module needs to examine the first
    // file to determine whether to output a BOM and whether to end output lines with `\r\n`
    // or just '\n'. Because all of the operations except `Union` need to handle the first
    // argument specially anyway, there's no great motivation to disguise this by using a
    // `Peekable` iterator.
    //
    let (first, rest, set_writer) = zet::io::prepare(args.files)?;
    if let Some(first_operand) = first {
        zet::do_calculation(args.op, &first_operand, rest, |harvest| set_writer.output(harvest))?;
    }
    Ok(())
}

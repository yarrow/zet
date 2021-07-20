use anyhow::Result;
use std::io;
fn main() -> Result<()> {
    let args = zet::args::parsed();

    // We use `(first, rest, set_writer)` because the `io` module needs to examine the first
    // file to determine whether to output a BOM and whether to end output lines with `\r\n`
    // or just '\n'. Because all of the operations except `Union` need to handle the first
    // argument specially anyway, there's no great motivation to disguise this by using a
    // `Peekable` iterator.
    //
    if atty::is(atty::Stream::Stdout) {
        zet::calculate::exec(args.op, &args.files, io::stdout().lock())?;
    } else {
        zet::calculate::exec(args.op, &args.files, io::BufWriter::new(io::stdout().lock()))?;
    };
    Ok(())
}

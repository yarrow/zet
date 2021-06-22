use std::io::Write;

fn main() -> Result<(), failure::Error> {
    let args = zet::args::parsed();

    let file_contents = zet::io::ContentsIter::from(args.files);
    let mut stdout = zet::io::stdout();

    zet::do_calculation(args.op, file_contents, {
        |iter| {
            for line in iter {
                stdout.write_all(line)?;
            }
            stdout.flush()?;
            Ok(())
        }
    })
}

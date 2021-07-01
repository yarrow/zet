fn main() -> Result<(), failure::Error> {
    let args = zet::args::parsed();

    let mut file_contents = zet::io::ContentsIter::from(args.files);
    if let Some(first_operand) = file_contents.next() {
        zet::do_calculation(args.op, &first_operand?, file_contents, zet::io::write_result)?;
    }
    Ok(())
}

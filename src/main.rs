fn main() -> Result<(), failure::Error> {
    let args = zet::args::parsed();

    let file_contents = zet::io::ContentsIter::from(args.files);

    zet::do_calculation(args.op, file_contents, zet::io::write_result)
}

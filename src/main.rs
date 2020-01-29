use std::process;

fn main() {
    let args = zet::args::parsed();

    let file_contents = zet::sio::ContentsIter::from(args.files);
    let mut stdout = zet::sio::stdout();

    if let Err(e) = zet::do_calculation(args.op, file_contents, &mut stdout) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

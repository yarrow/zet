use std::process;

fn main() {
    let args = setop::args::parsed();

    let file_contents = setop::sio::ContentsIter::from(args.files);
    let mut stdout = setop::sio::stdout();

    if let Err(e) = setop::do_calculation(args.op, file_contents, &mut stdout) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

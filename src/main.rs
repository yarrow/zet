#![cfg_attr(debug_assertions, allow(dead_code, unused))]

use failure::Error;

fn main() -> Result<(), Error> {
    let args = setop::args::parsed();

    let file_contents = setop::sio::ContentsIter::from(args.file);

    let stdout_for_locking = std::io::stdout();
    let mut output = stdout_for_locking.lock();

    setop::do_calculation(args.op, file_contents, &mut output)
}

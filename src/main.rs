#![cfg_attr(debug_assertions, allow(dead_code, unused))]

use failure::Error;

fn main() -> Result<(), Error> {
    let args = setop::args::parsed();
    let file_contents = setop::sio::ContentsIter::from(args.file);
    setop::do_calculation(args.op, file_contents)
}

#![cfg_attr(debug_assertions, allow(dead_code, unused))]

use failure::Error;

fn main() -> Result<(), Error> {
    let args = setop::args::parsed();
    setop::do_calculation(args.op, args.file)
}

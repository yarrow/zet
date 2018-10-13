#![cfg_attr(debug_assertions, allow(dead_code, unused))]

use failure::Error;

fn main() -> Result<(), Error> {
    let args = setop::args::parsed();
    setop::calculate(args.op, &args.file)
}

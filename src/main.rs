#![cfg_attr(debug_assertions, allow(dead_code, unused))]

use failure::Error;
use setop::args::parse_args;

fn main() -> Result<(), Error> {
    let args = parse_args();
    setop::calculate(args.op, args.file)
}

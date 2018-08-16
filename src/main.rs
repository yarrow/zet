#![cfg_attr(debug_assertions, allow(dead_code, unused))]

extern crate structopt;
extern crate failure;
use failure::Error;

extern crate setop;
use setop::args::parse_args;

fn main() -> Result<(), Error> {
    let args = parse_args();
    setop::calculate(args.op, args.file)
}

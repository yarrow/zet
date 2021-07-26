//! The `args` module parses the command line.
//! The `operands` module returns a `Vec[u8]` containing the contents of the
//! first operand and an iterator over the remaining operands.
//! The `operations` module performs the union, intersection, etc on the sets of
//! lines in each file.
//! The `set` module implements those sets, with file lines represented as keys
//! of a hash map (whose values are small bookkeeping types, varying by
//! operation).

#![deny(warnings)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports), warn(warnings))]
#![deny(unused_must_use)]
#![deny(clippy::all)]
#![allow(clippy::needless_return)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
pub mod args;
pub mod operands;
pub mod operations;
pub mod set;

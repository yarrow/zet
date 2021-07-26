//! The `calculate::exec` function is the kernel of the appliction.  The `args` module parses
//! the command line, and the `io` module hides I/O details.

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
pub mod calculate;
pub mod operands;
pub mod set;

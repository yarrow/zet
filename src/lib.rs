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
pub mod io;

use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use std::borrow::Cow;

/// The `LineIterator` type is used to return the value of a `SetExpression` `s`:
/// `s.iter()` returns an iterator over the lines (elements) of `s`.
///
pub(crate) type CowSet<'data, Bookkeeping> = IndexMap<Cow<'data, [u8]>, Bookkeeping, FxBuildHasher>;

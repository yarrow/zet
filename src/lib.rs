//! The `calculate::exec` function is the kernel of the appliction.  The `args` module parses
//! the command line, and the `io` module hides I/O details.

#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
#![deny(unused_must_use)]
#![deny(clippy::all)]
#![allow(clippy::needless_return)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![deny(missing_docs)]

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

//! Zet's overall flow is:
//! * Form a starting `ZetSet` from the lines of the first input file. Each line
//!   in the set is represented by an `IndexMap` key. The `IndexMap` value
//!   associated with each key is not part of the abstract set value but is
//!   used for operational bookkeeping. The type of these bookkeeping values
//!   depends on the operation being calculated and whether we're keeping track
//!   of the number of times each line occurs.
//! * Read the lines of each subsequent operand, updating the bookkeeping value
//!   as needed in order to decide whether to insert lines into or delete lines
//!   from the set.
//! * Output the lines of the resulting set, possibly annotated with a line count.
//!
//! Zet's structure is due to the following design decisions:
//! * We read the entire contents of the first input file into memory, so we can
//!   borrow the `IndexMap` key that represents each of its lines rather than
//!   allocating a `Vec<u8>` for each of them. This saves both time and memory,
//!   on the assumption that few lines in the first file are duplicates.
//! * We do *not* read the entire contents of subsequent files. This can cost us
//!   time in key allocation, but often saves both time and memory: `Intersect`
//!   and `Diff` never allocate, since they only remove lines from the set,
//!   while `Union`, `Single` and `Multiple` don't do extensive allocation
//!   in the fairly common case where the second and subsequent input files
//!   have few lines not already present in the first file.
//! * We start output with a Unicode byte order mark if and only the first input
//!   file begins with a byte order mark.
//! * We strip the line terminator (either `\r\n` or `\n`) from the end of each
//!   input line. On output, we use the line terminator found at the end of the
//!   first line of the first input file.
//! * We process all input files before doing any output. (This is not
//!   absolutely necessary for the `Union` operation — see the
//!   [huniq](https://crates.io/crates/huniq) command. But it is for all other
//!   Zet operations.)
//!
//! The `set` module provides a `ZetSet` structure the `zet_set_from` function,
//! which takes a `&[u8]` slice, a bookkeeping item used by the calling
//! operation, and a (possibly no-op) line counter. The call
//! ```ignore
//!     zet_set_from(slice, item, count)
//! ```
//! returns an initialized `ZetSet` with a representation of the set of (unique)
//! lines in the `u8` slice and some bookkeeping values:
//! * An `IndexMap` with keys (lines) borrowed from `slice` and initial
//!   bookkeeping values equal to `Bookkeeping{item, count}`.
//! * A field that indicates whether `slice` started with a byte order mark.
//! * A field that holds the line terminator to be used, taken from the first
//!   line of `slice`.
//!
//! The `count` field of a `Bookkeeping` struct is either an actual counter,
//! with an `increment()` method that increases it's `value()` by 1, or a
//! zero-sized fake counter whose `increment` method does nothing and whose
//! `value()` is always 0.
//!
//! `ZetSet` has `insert`, `retain`, and `get_mut` methods that act like
//! those methods on `HashMap` or `IndexMap`, except they expose only the `item`
//! field of their `Bookkeeping` values: `retain` takes a function that uses
//! items to decide whether to keep a line entry, `insert` takes an item, and
//! `get_mut` returns an item reference. The latter two process the `count`
//! field internally, initializing it for new entries and incrementing it for
//! already-seen entries.
//!
#![deny(
    warnings,
    clippy::all,
    clippy::cargo,
    clippy::pedantic,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_must_use
)]
#![allow(clippy::cargo)] // FIXME
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::struct_excessive_bools
)]
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

pub mod args;
pub mod help;
pub mod operands;
pub mod operations;
pub mod set;
pub mod styles;

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
//! The `set` module provides a `ZetSet` structure and extends `&[u8]` with the
//! `.to_zet_set_with(b)` method, which returns an initialized `ZetSet` with a
//! representation of the set of (unique) lines in the `u8` slice and some
//! guidance for eventual output. The value of `slice.to_zet_set_with(b)`
//! consists of:
//! * An `IndexMap` with keys (lines) borrowed from `slice` and initial
//!   bookkeeping values equal to `b`.
//! * A field that indicates whether `slice` started with a byte order mark.
//! * A field that holds the line terminator to be used, taken from the first
//!   line of `slice`.
//!
//! `ZetSet` exposes the `.insert` and `retain` methods of its internal
//! `IndexMap` operations to mutate the set of lines, and `.get_mut` to update
//! the bookkeeping value of a line. It provides an `.output_to` method to write
//! the lines of the set to an `io::Write` sink.
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

//! Zet's overall flow is:
//! * Form a starting `ZetSet` from the lines of the first input file. Each line
//!   in the set is represented by an `IndexMap` key. The `IndexMap` value
//!   associated with each key is not part of the abstract set value but is
//!   used for operational bookkeeping. The type of these bookkeeping values
//!   depends on the operation being calculated and whether we're keeping track
//!   of the number of times each line occurs or the number of files it occurs
//!   in.
//! * Read the lines of each subsequent operand, updating the bookkeeping value
//!   as needed in order to decide whether to insert lines into or delete lines
//!   from the set.
//! * Output the lines of the resulting set, possibly annotated with count of
//!   the number of times the line appears in the input or the number of files
//!   the line appears in.
//!
//! Zet's structure is due to the following design decisions:
//! * We read the entire contents of the first input file into memory, so we can
//!   borrow the `IndexMap` key that represents each of its lines rather than
//!   allocating a `Vec<u8>` for each of them. This saves both time and memory,
//!   on the assumption that few lines in the first file are duplicates.
//! * We do *not* read the entire contents of subsequent files. This can cost us
//!   time in key allocation, but often saves both time and memory: `Intersect`
//!   and `Diff` never allocate, since they only remove lines from the set, while
//!   the other operation won't do extensive allocation in the fairly common case
//!   where the second and subsequent input files have few lines not already
//!   present in the first file.
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
//! The `set` module provides the `ZetSet` structure. The `ZetSet::new` function
//! takes a `&[u8]` slice and a bookkeeping item used by the calling operation.
//! The call `ZetSet::new(slice, item)` returns an initialized `ZetSet` with:
//! * An `IndexMap` whose keys (lines) are borrowed from `slice` and initial
//!   bookkeeping values equal to `item`, and possibly updated if seen multiple
//!   times in the slice.
//! * A field that indicates whether `slice` started with a byte order mark.
//! * A field that holds the line terminator to be used, taken from the first
//!   line of `slice`.
//!
//! For a `ZetSet` `z`,
//! * `z.insert_or_update(operand, item)` uses `IndexMap`'s `entry` method to
//!   insert `item` as the value for lines in `operand` that were not already
//!   present in `z`, or to call `v.update_with(item)` on the bookkeeping item
//!   of lines that were present. Inserted lines are allocated, not borrowed, so
//!   `operand` need not outlive `z`.
//! * `z.update_if_present(operand, item)` calls `v.update_with(file_number)`
//!   on the bookkeeping item of lines in operand that are present in `z`,
//!   ignoring lines that are not already present.
//! * Finally, `z.retain(keep)` retains lines for which
//!   `keep(item.retention_value())` is true of the line's bookkeeping item.
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

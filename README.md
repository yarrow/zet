setop: Take the union, intersection, etc of files
=================================================

This is a command-line utility for doing set operations on files considered as
sets of lines. For instance, `setop union x y z` outputs the lines that occur in
any of `x`, `y`, or `z`. Two notes:

* Each output line occurs only once, because we're treating the files as sets
  and the lines as their elements.
* We do take the file structure into account in one respect: the lines are
  output in the same order as they are encountered. So `setop union x` prints
  out the lines of `x`, in order, with duplicates removed.

Here are the subcommands of `setop` and what they do:

* `setop union x y z` outputs the lines that occur in any of `x`, `y`, or `z`.
* `setop intersect x y z` outputs the lines that occur in all of `x`, `y`, and `z`.
* `setop diff x y z` outputs the lines that occur in `x` but not in `y` or `z`.
* `setop single x y z` outputs the lines that occur in exactly one of `x`, `y`,
  or `z`.
* `setop multiple x y z` outputs the lines that occur in two or more of `x`, `y`,
  and `z`.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

zet: Take the union, intersection, etc of files
=================================================

This is a command-line utility for doing set operations on files considered as
sets of lines. For instance, `zet union x y z` outputs the lines that occur in
any of `x`, `y`, or `z`, and `zet intersect x y z` those that occur in all of them.

[![ci](https://github.com/yarrow/zet/actions/workflows/ci.yml/badge.svg)](https://github.com/yarrow/zet/actions/workflows/ci.yml)

Here are the subcommands of `zet` and what they do:

* `zet union x y z` outputs the lines that occur in any of `x`, `y`, or `z`.
* `zet intersect x y z` outputs the lines that occur in all of `x`, `y`, and `z`.
* `zet diff x y z` outputs the lines that occur in `x` but not in `y` or `z`.
* `zet single x y z` outputs the lines that occur in exactly one of `x`, `y`,
  or `z`.
* `zet multiple x y z` outputs the lines that occur in two or more of `x`, `y`,
  and `z`.

Two notes:

* Each output line occurs only once, because we're treating the files as sets
  and the lines as their elements.
* We do take the file structure into account in one respect: the lines are
  output in the same order as they are encountered. So `zet union x` prints
  out the lines of `x`, in order, with duplicates removed.

## Plans

* Zet translates UTF-16LE and UTF-16BE files to UTF-8, and strips Byte Order Marks (BOMs) from UTF-8 files. Currently, its output never has a BOM; the plan is to prepend a BOM to Zet's output if and only if its first file argument begins with a BOM.

* Plan: Zet will strip all lines endings (`\r\n` or `\n`) from its inputs, so two input lines compare the same if their only difference is that one ends in `\r\n` and the other in `\r`. Zet will end each output line with `\r\n` if the first line of its first file argument ends in `\r\n`, and `\n` otherwise.

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

zet: Take the union, intersection, etc of files
=================================================

`zet` is a command-line utility for doing set operations on files considered as
sets of lines. For instance, `zet union x y z` outputs the lines that occur in
any of `x`, `y`, or `z`, and `zet intersect x y z` those that occur in all of them.

[![Build status](https://github.com/yarrow/zet/actions/workflows/ci.yml/badge.svg)](https://github.com/yarrow/zet/actions)
[![Crates.io](https://img.shields.io/crates/v/zet.svg)](https://crates.io/crates/zet)

Here are the subcommands of `zet` and what they do:

* `zet union x y z` outputs the lines that occur in any of `x`, `y`, or `z`.
* `zet intersect x y z` outputs the lines that occur in all of `x`, `y`, and `z`.
* `zet diff x y z` outputs the lines that occur in `x` but not in `y` or `z`.
* `zet single x y z` outputs the lines that occur exactly once in the entire input.
* `zet single --file x y z` outputs the lines that occur in exactly one of `x`,
  `y`, or `z`. (Output would include, say, a line that occurs, say, twice in `y`
  but not in `x` or `z`) 
* `zet multiple x y z` outputs the lines that occur more than once in the entire input.
* `zet multiple --files x y z` outputs the lines that occur in two or more of `x`, `y`,
  and `z` (but not a line that occurs twice in `y` but not in `x` or `z`).

The `--count-lines` flag makes `zet` show the number of times each line occurs in the input.
The `--count-files` flag shows the number of files each line occurs in.
The `-c` or `--count` flags act like `--count-lines`, unless `--files` is in effect, in which case they act like `--count-files`.

## Comparisons to other commands
Some `zet` subcommands are similar to traditional Unix commands:

  Zet           | Traditional
  ---           | -----------
  zet union     | uniq
  zet intersect | comm -12
  zet diff      | comm -23
  zet single    | uniq -u
  zet multiple  | uniq -d

Differences: `zet`'s input need not be sorted, and it can take multiple input
files (rather than just one (like `uniq`) or two (like `comm`).  For large
files, `zet` is 3 to 4 times faster than `uniq` and 7 to 8 times faster than
`comm`, but takes much more memory: `zet` reads its first file argument into
memory, and (for `union`, `single`, and `multiple`) allocates additional space
for each line encountered that wasn't in the first file. In contrast `uniq` and
`comm` take an essentially fixed amount of space, no matter how large the input,
since they depend on the input(s) being sorted.

So `zet` is faster until it runs into a memory limit, at which point it stops
working. (And `zet` has no `-i` or `-ignore-case` option, unlike `uniq` and
`comm`.)

The [`huniq`](https://crates.io/crates/huniq) command is slightly faster than
`zet union` and takes less memory, because it keeps only a hash of each line in
memory rather than the whole line. (In theory, `huniq` might fail to output a
line whose hash is the same as another, different, line). But `zet union
--count` is slightly faster than `huniq -c`, because `huniq -c` sorts its input
in order to count lines.

## Notes

* Each output line occurs only once, because we're treating the files as sets
  and the lines as their elements.
* We do take the file structure into account in one respect: the lines are
  output in the same order as they are encountered. So `zet union x` prints out
  the lines of `x`, in order, with duplicates removed.
* Zet translates UTF-16LE and UTF-16BE files to UTF-8, and ignores Byte Order
  Marks (BOMs) when comparing lines. It prepends a BOM to its output if and
  only if its first file argument begins with a BOM.
* Zet ignores all lines endings (`\r\n` or `\n`) when comparing lines, so two
  input lines compare the same if their only difference is that one ends in
  `\r\n` and the other in `\r`. Zet ends each output line with `\r\n` if the
  first line of its first file argument ends in `\r\n`, and `\n` otherwise (if
  the first line ends in `\n` or the first file has only one line and that line
  has no line terminator.)
* Zet reads its entire first input file into memory. Its memory usage is
  closely proportional to the size of its first input (`zet intersect` and `zet
  diff`) or the larger of the size of its first input and the size of its
  output (`zet union`, `zet single`, and `zet multiple`).

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

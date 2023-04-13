zet: Take the union, intersection, etc of files
=================================================

`zet` is a command-line utility for doing set operations on files considered as
sets of lines. For instance, `zet union x y z` outputs the lines that occur in
any of `x`, `y`, or `z`, `zet intersect x y z` those that occur in all of them, and `zet diff x y z` those that occur in `x` but not in `y` or `z`. `zet` prints each output line only once, and prints lines in the order of their first appearance in its input. 

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

## Example

Suppose you maintain three mailing lists on a site that lets you download membership lists as CSV files, and add new members by uploading a CSV file in the same format. You have three lists, `a`, `b`, and `c` that people have joined, and you want to create two new lists: `everyone`, whose membership should be those who have joined any of `a`, `b`, and `c`; and `big-fans`, whose membership should those who have signed up for all three of `a`, `b`, and `c`.

You've downloaded the membership lists `a`, `b`, and `c` to `a.csv`, `b.csv`, and `c.csv`. To create the membership list for `everyone` and `big-fans`, you can use `zet`:

```bash
zet union a.csv b.csv c.csv > everyone.csv
zet intersect a.csv b.csv c.csv > big-fans.csv
```

Alas, by the time you create `everyone` and `big-fans`, new people have joined the `a`, `b`, and `c` lists. So you download the current membership of those lists to `a-now.csv`, `b-now.csv`, and `c-now.csv`.  You create `new-everyone.csv` and `new-big-fans.csv`, containing the membership records of people who should be added to the `everyone` list and `big-fan` list respectively:

```bash
zet union a-now.csv b-now.csv c-now.csv | zet diff - everyone.csv > new-everyone.csv
zet intersect a-now.csv b-now.csv c-now.csv | zet diff - big-fans.csv > new-big-fans.csv
```

## Comparisons to other commands
Some `zet` subcommands are similar to traditional Unix commands:

  Zet           | Traditional
  ---           | -----------
  zet union     | uniq
  zet intersect | comm -12
  zet diff      | comm -23
  zet single    | uniq -u
  zet multiple  | uniq -d

Differences:
* `zet`'s input need not be sorted, and it outputs lines in the order of their
  first appearance in the input. It can take multiple input files (rather than
  just one (like `uniq`) or two (like `comm`).
* `zet` has no `-i` or `-ignore-case` option, unlike `uniq` and `comm`.For
  large files, `zet` is about 4.5 times faster than `uniq` and 10 times faster
  than `comm` (see [benchmark details](doc/zet-vs-other-commands.md)). But
  `zet` takes much more memory than `uniq` or `comm`: `zet` reads its first
  file argument into memory, and (for `union`, `single`, and `multiple`)
  allocates additional space for each line encountered that wasn't in the first
  file. In contrast `uniq` and `comm` take an essentially fixed amount of
  space, no matter how large the input, since they depend on the input(s) being
  sorted. So `zet` is faster until it runs into a memory limit, at which point
  it stops working.

The [`huniq`](https://crates.io/crates/huniq) command is slightly faster than
`zet union` and takes less memory, because it keeps only a hash of each line in
memory rather than the whole line. (In theory, `huniq` might fail to output a
line whose hash is the same as another, different, line). But `zet union
--count` is slightly faster than `huniq -c`, because `huniq -c` sorts its input
in order to count lines.

## Notes

* As stated above, each output line occurs only once, and the lines are output
  in the same order as they are encountered.
* When no file path is given on the command line, zet reads from standard
  input.
* When a file argument is `-`, `zet` reads from standard input rather than the
  file named `-`.
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

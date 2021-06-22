zet: Take the union, intersection, etc of files
=================================================

This is a command-line utility for doing set operations on files considered as
sets of lines. For instance, `zet union x y z` outputs the lines that occur in
any of `x`, `y`, or `z`. Two notes:

* Each output line occurs only once, because we're treating the files as sets
  and the lines as their elements.
* We do take the file structure into account in one respect: the lines are
  output in the same order as they are encountered. So `zet union x` prints
  out the lines of `x`, in order, with duplicates removed.

Here are the subcommands of `zet` and what they do:

* `zet union x y z` outputs the lines that occur in any of `x`, `y`, or `z`.
* `zet intersect x y z` outputs the lines that occur in all of `x`, `y`, and `z`.
* `zet diff x y z` outputs the lines that occur in `x` but not in `y` or `z`.
* `zet single x y z` outputs the lines that occur in exactly one of `x`, `y`,
  or `z`.
* `zet multiple x y z` outputs the lines that occur in two or more of `x`, `y`,
  and `z`.

## Limitations

* Zet currently doesn't work with UTF-16LE and UTF-16BE file encodings, and doesn't work well with UTF-8 files that start with a Byte Order Mark. So it's currently not a good fit for Windows. (The plan is to change this!)
* In some files, the last line lacks an end of line marker. Zet will add that marker (so such a line can be usefully compared to a line in the middle of a file), using `\r\n` if the first line in the file ended that way, or `\n` if not. If the file has only a single line, not terminated, then Zet will use `\n`, which could be a problem if other files use `\r\n`.

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

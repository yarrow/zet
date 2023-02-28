# Change Log

## [Unreleased]

- Self-indulgent recreation of (part of) clap's `help` feature, because I like the `clap 4`'s help format, but really miss the colored (rather than gray-scale) help.

## [0.2.6] - 2023-02-02

- Abandon trying to have a Minimum Supported Rust Version (maybe once we're 1.0?)
- Use cargo-dist to create the release
- Move `for_byte_lines` from `NextOperand` to a trait (thanks to [ysthakur] for the suggestion)

## [0.2.5] - 2022-11-10

## Changed
- Bump Minimum Supported Rust Version to 1.64.0
- Switch from `failure` to `anyhow`
- Performance enhancements:
    - Use `Cow` keys for `UnionSet` and `CountedSet` so we can borrow the lines of
      the first file rather than allocating them
    - If `line` is in a `CountedSet`, don't allocate a key
    - Use `FxHash` — averages 10-15% faster on large files
    - Convert `Diff` and `Union` to use `CowSet`
    - Convert `Single`, `Multiple`, and `Intersect` to by-line algorithms
    - No longer create map/set for args after the 1st
- Refactor and expand internal documentation.
- Change Single/Multiple code to use a single NonZeroUsize operand ID rather than
  two u32 IDs

## [0.2.0] - 2021-07-03

## Changed
- Zet looks for Byte Order Marks in UTF-8, UTF-16LE and UTF-16BE files,
  translating UTF-16LE and UTF-16BE to UTF-8. It outputs a (UTF-8) Byte Order
  Mark if and only if it finds one in its first file argument.
- Zet strips off the line terminator (`\n` or `\r\n`) from each input line. On
  output, it uses the line terminator found in the first line of its first file
  argument (or `\n` if the first file consists of a single line with no
  terminator).

## [0.1.1] - 2021-06-14

## Fixed
- Upgrade from yanked dependencies

## 0.1.0 - 2021-06-14

Initial release

[Unreleased]: https://github.com/yarrow/zet/compare/0.2.6...HEAD
[0.2.6]: https://github.com/yarrow/zet/compare/0.2.5...v0.2.6
[0.2.5]: https://github.com/yarrow/zet/compare/0.2.0...0.2.5
[0.2.0]: https://github.com/yarrow/zet/compare/v0.1.1...0.2.0
[0.1.1]: https://github.com/yarrow/zet/compare/v0.1.0...v0.1.1
[ysthakur]:https://github.com/ysthakur

# Change Log

## [Unreleased]
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

[Unreleased]: https://github.com/yarrow/zet/compare/0.2.0...HEAD
[0.2.0]: https://github.com/yarrow/zet/compare/v0.1.1...0.2.0
[0.1.1]: https://github.com/yarrow/zet/compare/v0.1.0...v0.1.1

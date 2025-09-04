# lexlib.rs

A miscellaneous Rust utility library and an archive of any useful code I write
but don't have a home for.

## Notable features

* [`Scanner`](src/text/scanner.rs) and
  [`ScannerLite`](src/text/scanner_lite.rs), specialized iterators for scanning
  and parsing text.
* [`Utf8ChunkReader` and `Utf8CharReader`](src/io/utf8.rs) which can be used to
  process large UTF-8 files and byte streams
  [“online”](https://en.wikipedia.org/wiki/Online_algorithm) with a fixed-length
  buffer.
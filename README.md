jcfmt
=====

[![jcfmt](https://img.shields.io/crates/v/jcfmt.svg)](https://crates.io/crates/jcfmt)
[![Documentation](https://docs.rs/jcfmt/badge.svg)](https://docs.rs/jcfmt)
[![Actions Status](https://github.com/sile/jcfmt/workflows/CI/badge.svg)](https://github.com/sile/jcfmt/actions)
![License](https://img.shields.io/crates/l/jcfmt)

A command-line tool to format JSONC (JSON with Comments) text.

Usage
-----

```console
$ jcfmt -h
A command-line tool to format JSONC (JSON with Comments) text

Usage: jcfmt [OPTIONS]

Options:
      --version        Print version
  -h, --help           Print help ('--help' for full help, '-h' for summary)
  -s, --strip-comments Remove all comments from the JSON output

$ echo '{/*foo*/"bar":"baz"}' | jcfmt
{ /*foo*/
  "bar": "baz"
}
```

MEMO:
- formatting features
  - preserving input printable char orders (only whitespaces are adjusted)
  - consider input newline position (if array or map contains a newline, the direct children are formatted by multilinemode)
  - preseving a newline if there more than two succesive newlines in the input
  - etc
- unsupported (or notice)
  - dones not allow trailing commas


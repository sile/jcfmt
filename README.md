jcfmt
=====

[![jcfmt](https://img.shields.io/crates/v/jcfmt.svg)](https://crates.io/crates/jcfmt)
[![Documentation](https://docs.rs/jcfmt/badge.svg)](https://docs.rs/jcfmt)
[![Actions Status](https://github.com/sile/jcfmt/workflows/CI/badge.svg)](https://github.com/sile/jcfmt/actions)
![License](https://img.shields.io/crates/l/jcfmt)

`jcfmt` is a command-line tool to format JSONC (JSON with Comments) text.

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

Installation
------------

```console
$ cargo install jcfmt
```

JSONC
-----

JSONC (JSON with Comments) extends standard JSON by allowing comments,
making configuration files more readable and maintainable. 

While there are various JSONC implementations,
`jcfmt` supports the two most common comment styles:

- **Line comments**: `//`
  - Everything after `//` to the end of the line is treated as a comment
- **Block comments**: `/* */`
  - Multi-line comments that can span across multiple lines

### Example

```jsonc
{
  // This is a line comment
  "name": "example",

  /* This is a block comment
     that spans multiple lines */
  "version": "1.0.0",

  "config": {
    "debug": true, // Another line comment
    "port": 8080
  }
}
```

Formatting Behavior
-------------------

MEMO:
- formatting features
  - preserving input printable char orders (only whitespaces are adjusted)
  - consider input newline position (if array or map contains a newline, the direct children are formatted by multilinemode)
  - preseving a newline if there more than two succesive newlines in the input
  - etc


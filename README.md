jcfmt
=====

[![jcfmt](https://img.shields.io/crates/v/jcfmt.svg)](https://crates.io/crates/jcfmt)
[![Documentation](https://docs.rs/jcfmt/badge.svg)](https://docs.rs/jcfmt)
[![Actions Status](https://github.com/sile/jcfmt/workflows/CI/badge.svg)](https://github.com/sile/jcfmt/actions)
![License](https://img.shields.io/crates/l/jcfmt)

`jcfmt` is a command-line tool to format JSONC (JSON with Comments) text.

Before:
```jsonc
{"name":"example", // App name

// config and features
"config": {"debug":true, "port":8080/* TODO: fix later */},
"features": ["auth","logging"]}
```

After:
```jsonc
{
  "name": "example", // App name

  // config and features
  "config": {
    "debug": true,
    "port": 8080 /* TODO: fix later */
  },
  "features": ["auth", "logging"]
}
```

Key Features
------------

- **Comment-aware JSON formatting**:
  - Supports both line comments (`//`) and block comments (`/* */`)
- **Character preservation**:
  - Only whitespace is adjusted
  - All printable characters maintain their original order
- **Content-aware newline insertion**:
  - Uses multiline formatting when input contains newlines or comments within arrays and objects
- **Blank line preservation**:
  - Maintains blank lines when there are multiple successive newlines in input

Installation
------------

```console
$ cargo install jcfmt

$ jcfmt -h
A command-line tool to format JSONC (JSON with Comments) text

Usage: jcfmt [OPTIONS]

Options:
      --version        Print version
  -h, --help           Print help ('--help' for full help, '-h' for summary)
  -s, --strip-comments Remove all comments from the JSON output
```

Examples
--------

```console
// Simple example
$ echo '{/*foo*/"bar":"baz"}' | jcfmt
{ /*foo*/
  "bar": "baz"
}

// Complex example
$ cat example.jsonc
{"name":"example", // App name

/* config and
   features */
"config": {"debug": true, "port": 8080 /* TODO: fix later */},
"features": ["auth", "logging"]
}

$ cat example.jsonc | jcfmt
{
  "name": "example", // App name

  /* config and
     features */
  "config": {
    "debug": true,
    "port": 8080 /* TODO: fix later */
  },
  "features": ["auth", "logging"]
}

// The `--strip-comments` flag produces plain JSON output
$ cat example.jsonc | jcfmt --strip-comments
{
  "name": "example",

  "config": {"debug": true, "port": 8080},
  "features": ["auth", "logging"]
}
```

About JSONC
-----------

JSONC (JSON with Comments) extends standard JSON by allowing comments,
making configuration files more readable and maintainable.

While there are various JSONC implementations,
`jcfmt` supports the two most common comment styles:

- **Line comments**: `//`
  - Everything after `//` to the end of the line is treated as a comment
- **Block comments**: `/* */`
  - Multi-line comments that can span across multiple lines


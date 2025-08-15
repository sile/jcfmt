use std::collections::BTreeMap;
use std::io::{StdoutLock, Write};
use std::ops::Range;

const INDENT_SIZE: usize = 2;

fn main() -> noargs::Result<()> {
    let mut args = noargs::raw_args();

    args.metadata_mut().app_name = env!("CARGO_PKG_NAME");
    args.metadata_mut().app_description = env!("CARGO_PKG_DESCRIPTION");

    if noargs::VERSION_FLAG.take(&mut args).is_present() {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    noargs::HELP_FLAG.take_help(&mut args);

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let text = std::io::read_to_string(std::io::stdin())?;
    let (json, comment_ranges) = nojson::RawJson::parse_jsonc(&text)?;
    let stdout = std::io::stdout();
    let mut formatter = Formatter::new(&text, comment_ranges, stdout.lock());
    formatter.format(json.value())?;

    Ok(())
}

#[derive(Debug)]
struct Formatter<'a> {
    text: &'a str,
    comment_ranges: BTreeMap<usize, usize>,
    stdout: StdoutLock<'a>,
    level: usize,
}

impl<'a> Formatter<'a> {
    fn new(text: &'a str, comment_ranges: Vec<Range<usize>>, stdout: StdoutLock<'a>) -> Self {
        Self {
            text,
            comment_ranges: comment_ranges
                .into_iter()
                .map(|r| (r.start, r.end))
                .collect(),
            stdout,
            level: 0,
        }
    }

    fn format(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_value(value)?;
        writeln!(self.stdout)?;
        Ok(())
    }

    fn format_value(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        match value.kind() {
            nojson::JsonValueKind::Null
            | nojson::JsonValueKind::Boolean
            | nojson::JsonValueKind::Integer
            | nojson::JsonValueKind::Float
            | nojson::JsonValueKind::String => write!(self.stdout, "{}", value.as_raw_str())?,
            nojson::JsonValueKind::Array => self.format_array(value)?,
            nojson::JsonValueKind::Object => self.format_object(value)?,
        }
        Ok(())
    }

    fn format_array(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        write!(self.stdout, "[")?;
        self.level += 1;

        let newline = self.is_newline_needed(value);
        for (i, element) in value.to_array().expect("bug").enumerate() {
            if i > 0 {
                write!(self.stdout, ",")?;
            }
            if newline {
                self.indent()?;
            }
            self.format_value(element)?;
        }
        self.level -= 1;
        if newline {
            self.indent()?
        }
        write!(self.stdout, "]",)?;
        Ok(())
    }

    fn format_object(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        write!(self.stdout, "{{")?;
        self.level += 1;

        let newline = self.is_newline_needed(value);
        for (i, (key, value)) in value.to_object().expect("bug").enumerate() {
            if i > 0 {
                write!(self.stdout, ",")?;
            }
            if newline {
                self.indent()?;
            } else {
                write!(self.stdout, " ")?;
            }

            self.format_value(key)?;
            write!(self.stdout, ": ")?;
            self.format_value(value)?;

            if !newline {
                write!(self.stdout, " ")?;
            }
        }
        self.level -= 1;
        if newline {
            self.indent()?
        }
        write!(self.stdout, "}}")?;
        Ok(())
    }

    fn is_newline_needed(&self, value: nojson::RawJsonValue<'_, '_>) -> bool {
        match value.kind() {
            nojson::JsonValueKind::Null
            | nojson::JsonValueKind::Boolean
            | nojson::JsonValueKind::Integer
            | nojson::JsonValueKind::Float
            | nojson::JsonValueKind::String => false,
            nojson::JsonValueKind::Array => {
                self.is_comment_included(value)
                    || value
                        .to_array()
                        .expect("bug")
                        .enumerate()
                        .any(|(i, value)| i > 0 || self.is_newline_needed(value))
            }
            nojson::JsonValueKind::Object => {
                self.is_comment_included(value)
                    || value
                        .to_object()
                        .expect("bug")
                        .enumerate()
                        .any(|(i, (_, value))| i > 0 || self.is_newline_needed(value))
            }
        }
    }

    fn is_comment_included(&self, value: nojson::RawJsonValue<'_, '_>) -> bool {
        let start = value.position();
        let end = start + value.as_raw_str().len();
        self.comment_ranges.range(start..end).next().is_some()
    }

    fn indent(&mut self) -> std::io::Result<()> {
        write!(
            self.stdout,
            "\n{:width$}",
            "",
            width = self.level * INDENT_SIZE
        )
    }
}

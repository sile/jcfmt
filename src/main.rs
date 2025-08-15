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
    text_position: usize,
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
            text_position: 0,
        }
    }

    fn format(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_value(value)?;
        self.format_trailing_comment(self.text.len())?;

        writeln!(self.stdout)?;

        // TODO: write normal comment if need
        Ok(())
    }

    fn format_value(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_normal_comment(value)?;
        match value.kind() {
            nojson::JsonValueKind::Null
            | nojson::JsonValueKind::Boolean
            | nojson::JsonValueKind::Integer
            | nojson::JsonValueKind::Float
            | nojson::JsonValueKind::String => write!(self.stdout, "{}", value.as_raw_str())?,
            nojson::JsonValueKind::Array => self.format_array(value)?,
            nojson::JsonValueKind::Object => self.format_object(value)?,
        }
        self.text_position = value.position() + value.as_raw_str().len();
        Ok(())
    }

    fn format_normal_comment(
        &mut self,
        _value: nojson::RawJsonValue<'_, '_>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    fn format_trailing_comment(&mut self, next_position: usize) -> std::io::Result<()> {
        loop {
            let Some((comment_start, comment_end)) = self
                .comment_ranges
                .range(self.text_position..next_position)
                .next()
                .map(|x| (*x.0, *x.1))
            else {
                return Ok(());
            };
            if self.text[self.text_position..comment_end].contains('\n') {
                return Ok(());
            };

            // TODO: consider multi-line block comment (should be treated as a normal comment)

            write!(self.stdout, " {}", &self.text[comment_start..comment_end])?;
            self.comment_ranges.remove(&comment_start);
        }
    }

    fn format_array(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        write!(self.stdout, "[")?;
        self.level += 1;

        // TODO: write comment if need

        let newline = self.is_newline_needed(value);
        for (i, element) in value.to_array().expect("bug").enumerate() {
            if i > 0 {
                write!(self.stdout, ",")?;
                self.format_trailing_comment(element.position())?;
            }
            if newline {
                self.indent()?;
            }
            self.format_value(element)?;
        }
        self.format_trailing_comment(value.position() + value.as_raw_str().len())?;

        self.level -= 1;
        if newline {
            self.indent()?
        }

        // TODO: write comment if need

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
                self.format_trailing_comment(key.position())?;
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
        self.format_trailing_comment(value.position() + value.as_raw_str().len())?;

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

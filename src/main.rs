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

    let strip_comments = noargs::flag("strip-comments")
        .short('s')
        .take(&mut args)
        .is_present();

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let text = std::io::read_to_string(std::io::stdin())?;
    let (json, mut comment_ranges) = nojson::RawJson::parse_jsonc(&text)?;
    if strip_comments {
        comment_ranges.clear();
    }

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
    multiline_mode: bool,
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
            multiline_mode: false,
        }
    }

    fn format(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_value(value)?;
        self.format_trailing_comment(self.text.len())?;
        writeln!(self.stdout)?;
        if !self.comment_ranges.is_empty() {
            self.blank_line(self.text.len())?;
            self.format_leading_comment(self.text.len())?;
        }
        Ok(())
    }

    fn format_value(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_leading_comment(value.position())?;
        if self.multiline_mode {
            self.format_trailing_comment(value.position())?;
            self.blank_line(value.position())?;
            self.indent_value(value)?;
            self.format_leading_comment(self.text.len())?;
        }
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

    fn format_symbol(&mut self, ch: char) -> std::io::Result<()> {
        write!(self.stdout, "{ch}")?;
        let position = self.text[self.text_position..].find(ch).expect("bug");
        self.text_position += position + 1;
        Ok(())
    }

    fn contains_comment(&self, position: usize) -> bool {
        self.comment_ranges.range(..position).next().is_some()
    }

    fn format_leading_comment(&mut self, position: usize) -> std::io::Result<()> {
        loop {
            let Some((comment_start, comment_end)) = self
                .comment_ranges
                .range(..position)
                .next()
                .map(|x| (*x.0, *x.1))
            else {
                return Ok(());
            };

            let comment = &self.text[comment_start..comment_end];
            if comment.starts_with("//") {
                write!(self.stdout, "{comment}")?;
                self.indent()?;
            } else {
                for (i, line) in comment.lines().enumerate() {
                    if i == 0 {
                        write!(self.stdout, "{}", line.trim())?;
                    } else {
                        self.indent()?;
                        write!(self.stdout, "   {}", line.trim())?;
                    }
                }
                self.indent()?;
            }
            self.comment_ranges.remove(&comment_start);
        }
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
            }

            let comment = &self.text[comment_start..comment_end];
            write!(self.stdout, " {comment}")?;
            self.comment_ranges.remove(&comment_start);
            self.text_position = comment_end;
        }
    }

    fn format_array(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_symbol('[')?;
        self.level += 1;

        let old_multiline_mode = self.multiline_mode;
        self.multiline_mode = self.is_newline_needed(value);
        for (i, element) in value.to_array().expect("bug").enumerate() {
            if i > 0 {
                self.format_symbol(',')?;
                self.format_trailing_comment(element.position())?;
                if !self.multiline_mode {
                    write!(self.stdout, " ")?;
                }
            }
            self.format_value(element)?;
        }
        let close_position = value.position() + value.as_raw_str().len();
        self.format_trailing_comment(close_position)?;

        self.level -= 1;
        if self.multiline_mode {
            self.indent()?;
            if self.contains_comment(close_position) {
                write!(self.stdout, "{:width$}", "", width = INDENT_SIZE)?;
            }
        }
        self.format_leading_comment(close_position)?;

        self.format_symbol(']')?;
        self.multiline_mode = old_multiline_mode;
        Ok(())
    }

    fn format_object(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.format_symbol('{')?;
        self.level += 1;

        let old_multiline_mode = self.multiline_mode;
        self.multiline_mode = self.is_newline_needed(value);
        for (i, (key, value)) in value.to_object().expect("bug").enumerate() {
            if i > 0 {
                self.format_symbol(',')?;
                self.format_trailing_comment(key.position())?;
                if !self.multiline_mode {
                    write!(self.stdout, " ")?;
                }
            }

            self.format_value(key)?;
            self.format_symbol(':')?;
            write!(self.stdout, " ")?; // TODO
            self.format_value(value)?;
        }
        let close_position = value.position() + value.as_raw_str().len();
        self.format_trailing_comment(close_position)?;

        self.level -= 1;
        if self.multiline_mode {
            self.indent()?;
            if self.contains_comment(close_position) {
                write!(self.stdout, "{:width$}", "", width = INDENT_SIZE)?;
            }
        }

        self.format_leading_comment(close_position)?;

        self.format_symbol('}')?;
        self.multiline_mode = old_multiline_mode;
        Ok(())
    }

    fn is_newline_needed(&self, value: nojson::RawJsonValue<'_, '_>) -> bool {
        self.is_comment_included(value) || self.is_newline_included(value)
    }

    fn is_comment_included(&self, value: nojson::RawJsonValue<'_, '_>) -> bool {
        let start = value.position();
        let end = start + value.as_raw_str().len();
        self.comment_ranges.range(start..end).next().is_some()
    }

    fn is_newline_included(&self, value: nojson::RawJsonValue<'_, '_>) -> bool {
        let start = value.position();
        let end = start + value.as_raw_str().len();
        self.text[start..end].contains('\n')
    }

    fn blank_line(&mut self, position: usize) -> std::io::Result<()> {
        let text = &self.text[self.text_position..position];
        let Some(offset) = text.find('\n') else {
            return Ok(());
        };
        if !text[offset + 1..].contains('\n') {
            return Ok(());
        }
        writeln!(self.stdout)?;
        Ok(())
    }

    fn indent_value(&mut self, _value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.indent()
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

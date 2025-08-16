use std::collections::BTreeMap;
use std::io::Write;
use std::num::NonZeroUsize;
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
        .doc("Remove all comments from the JSON output")
        .take(&mut args)
        .is_present();

    if let Some(help) = args.finish()? {
        print!("{help}");
        return Ok(());
    }

    let text = std::io::read_to_string(std::io::stdin())?;
    let (json, mut comment_ranges) =
        nojson::RawJson::parse_jsonc(&text).map_err(|e| format_json_parse_error(&text, e))?;
    if strip_comments {
        comment_ranges.clear();
    }

    let stdout = std::io::stdout();
    let mut formatter = Formatter::new(&text, comment_ranges, stdout.lock());
    formatter.format(json.value())?;

    Ok(())
}

#[derive(Debug)]
struct Formatter<'a, W> {
    text: &'a str,
    comment_ranges: BTreeMap<usize, usize>,

    writer: W,
    level: usize,
    text_position: usize,
    multiline_mode: bool,
}

impl<'a, W: Write> Formatter<'a, W> {
    fn new(text: &'a str, comment_ranges: Vec<Range<usize>>, writer: W) -> Self {
        Self {
            text,
            comment_ranges: comment_ranges
                .into_iter()
                .map(|r| (r.start, r.end))
                .collect(),
            writer,
            level: 0,
            text_position: 0,
            multiline_mode: false,
        }
    }

    fn format(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        self.multiline_mode = self.is_newline_needed(value);
        self.format_value(value)?;
        self.format_comments(self.text.len())?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn format_value(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        if self.multiline_mode {
            self.format_comments(value.position())?;
            self.indent(value.position())?;
        }
        self.format_value_content(value)?;
        Ok(())
    }

    fn format_member_value(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        if self.contains_comment(value.position()) {
            self.format_comments(value.position())?;
            self.indent(value.position())?;
        } else {
            write!(self.writer, " ")?;
        }
        self.format_value_content(value)?;
        Ok(())
    }

    fn format_value_content(&mut self, value: nojson::RawJsonValue<'_, '_>) -> std::io::Result<()> {
        match value.kind() {
            nojson::JsonValueKind::Null
            | nojson::JsonValueKind::Boolean
            | nojson::JsonValueKind::Integer
            | nojson::JsonValueKind::Float
            | nojson::JsonValueKind::String => write!(self.writer, "{}", value.as_raw_str())?,
            nojson::JsonValueKind::Array => self.format_array(value)?,
            nojson::JsonValueKind::Object => self.format_object(value)?,
        }
        self.text_position = value.position() + value.as_raw_str().len();
        Ok(())
    }

    fn format_symbol(&mut self, ch: char) -> std::io::Result<()> {
        let mut position =
            self.text_position + self.text[self.text_position..].find(ch).expect("bug") + 1;
        while self
            .comment_ranges
            .range(..position)
            .next_back()
            .is_some_and(|(_, &end)| position < end)
        {
            position += self.text[position..].find(ch).expect("bug") + 1;
        }

        if (self.multiline_mode && matches!(ch, ']' | '}')) || self.contains_comment(position) {
            self.format_comments(position)?;
            if matches!(ch, ']' | '}') {
                self.text_position = position - 1;
            }
            self.indent(position)?;
        }

        write!(self.writer, "{ch}")?;
        if !self.multiline_mode && matches!(ch, ',') {
            write!(self.writer, " ")?;
        }
        self.text_position = position;
        Ok(())
    }

    fn contains_comment(&self, position: usize) -> bool {
        self.comment_ranges.range(..position).next().is_some()
    }

    fn format_comments(&mut self, position: usize) -> std::io::Result<()> {
        self.format_trailing_comment(position)?;
        self.format_leading_comment(position)?;
        Ok(())
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

            self.indent(comment_start)?;
            self.text_position = comment_start;
            let comment = &self.text[comment_start..comment_end];
            if comment.starts_with("//") {
                write!(self.writer, "{}", comment.trim_end())?;
            } else {
                let after_indent = self.level * INDENT_SIZE;
                let before_indent = self.text[..comment_start]
                    .lines()
                    .next_back()
                    .expect("bug")
                    .len();
                for (i, mut line) in comment.lines().enumerate() {
                    if i == 0 {
                        write!(self.writer, "{}", line.trim())?;
                    } else if let Some(delta) = after_indent.checked_sub(before_indent) {
                        write!(self.writer, "\n{:width$}", line.trim_end(), width = delta)?;
                    } else {
                        let delta = before_indent - after_indent;
                        for _ in 0..delta {
                            if let Some(l) = line.strip_prefix(' ') {
                                line = l;
                            } else {
                                break;
                            };
                        }
                        write!(self.writer, "\n{}", line.trim_end())?;
                    }
                }
            }
            self.comment_ranges.remove(&comment_start);
            self.text_position = comment_end;
        }
    }

    fn format_trailing_comment(&mut self, next_position: usize) -> std::io::Result<()> {
        if self.text_position == 0 {
            return Ok(());
        };
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

            let comment = self.text[comment_start..comment_end].trim_end();
            write!(self.writer, " {comment}")?;
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
            }
            self.format_value(element)?;
        }
        let close_position = value.position() + value.as_raw_str().len();
        self.format_comments(close_position)?;

        self.level -= 1;
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
            }

            self.format_value(key)?;
            self.format_symbol(':')?;
            self.format_member_value(value)?;
        }
        let close_position = value.position() + value.as_raw_str().len();
        self.format_comments(close_position)?;

        self.level -= 1;
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
        let Some(offset) = self.text[self.text_position..position].find('\n') else {
            return Ok(());
        };
        self.text_position += offset + 1;

        let Some(offset) = self.text[self.text_position..position].find('\n') else {
            return Ok(());
        };
        self.text_position += offset + 1;

        writeln!(self.writer)?;

        Ok(())
    }

    fn indent(&mut self, position: usize) -> std::io::Result<()> {
        if self.text_position == 0 {
            return Ok(());
        }
        self.blank_line(position)?;
        write!(
            self.writer,
            "\n{:width$}",
            "",
            width = self.level * INDENT_SIZE
        )
    }
}

fn format_json_parse_error(text: &str, error: nojson::JsonParseError) -> String {
    let (line_num, column_num) = error
        .get_line_and_column_numbers(text)
        .unwrap_or((NonZeroUsize::MIN, NonZeroUsize::MIN));

    let line = error.get_line(text).unwrap_or("");

    let prev_line = if line_num.get() == 1 {
        None
    } else {
        text.lines().nth(line_num.get() - 2)
    };

    let (display_line, display_column) = format_line_around_position(line, column_num.get());
    let prev_display_line = prev_line.map(|prev| {
        let (truncated, _) = format_line_around_position(prev, column_num.get());
        truncated
    });

    format!(
        "{error}\n\nINPUT:{}\n{line_num:4} |{display_line}\n     |{:>column$} error",
        if let Some(prev) = prev_display_line {
            format!("\n     |{prev}")
        } else {
            "".to_owned()
        },
        "^",
        column = display_column
    )
}

fn format_line_around_position(line: &str, column_pos: usize) -> (String, usize) {
    const MAX_ERROR_LINE_CHARS: usize = 80;

    let chars: Vec<char> = line.chars().collect();
    let max_context = MAX_ERROR_LINE_CHARS / 2;

    let error_pos = column_pos.saturating_sub(1).min(chars.len());
    let start_pos = error_pos.saturating_sub(max_context);
    let end_pos = (error_pos + max_context + 1).min(chars.len());

    let mut result = String::new();
    let mut new_column_pos = error_pos - start_pos + 1;

    if start_pos > 0 {
        result.push_str("...");
        new_column_pos += 3;
    }

    result.push_str(&chars[start_pos..end_pos].iter().collect::<String>());

    if end_pos < chars.len() {
        result.push_str("...");
    }

    (result, new_column_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(text: &str) -> String {
        let (json, comment_ranges) = nojson::RawJson::parse_jsonc(text).expect("bug");
        let mut buf = Vec::new();
        let mut formatter = Formatter::new(&text, comment_ranges, &mut buf);
        formatter.format(json.value()).expect("bug");
        String::from_utf8(buf).expect("bug")
    }

    #[test]
    fn literals() {
        assert_eq!(format(" null  "), "null\n");
        assert_eq!(format(" \t\n false\n\n  "), "false\n");
        assert_eq!(format(" 1\n "), "1\n");
        assert_eq!(format(" \n\"foo\" "), "\"foo\"\n");
    }

    #[test]
    fn empty_containers() {
        assert_eq!(format("[]"), "[]\n");
        assert_eq!(format("{}"), "{}\n");
        assert_eq!(format(" [ ] "), "[]\n");
        assert_eq!(format(" { } "), "{}\n");
    }

    #[test]
    fn arrays() {
        assert_eq!(format("[1, 2, 3]"), "[1, 2, 3]\n");
        assert_eq!(format("[1,2,3]"), "[1, 2, 3]\n");
        assert_eq!(format("[ 1 , 2 , 3 ]"), "[1, 2, 3]\n");

        // Multiline arrays
        assert_eq!(format("[\n  1,\n  2,\n  3\n]"), "[\n  1,\n  2,\n  3\n]\n");

        // Nested arrays
        assert_eq!(format("[[1, 2], [3, 4]]"), "[[1, 2], [3, 4]]\n");
        assert_eq!(
            format("[\n  [1, 2],\n  [3, 4]\n]"),
            "[\n  [1, 2],\n  [3, 4]\n]\n"
        );
    }

    #[test]
    fn objects() {
        assert_eq!(format("{\"a\": 1}"), "{\"a\": 1}\n");
        assert_eq!(format("{\"a\":1}"), "{\"a\": 1}\n");
        assert_eq!(format("{ \"a\" : 1 }"), "{\"a\": 1}\n");

        // Multiple properties
        assert_eq!(format("{\"a\": 1, \"b\": 2}"), "{\"a\": 1, \"b\": 2}\n");

        // Multiline objects
        assert_eq!(
            format("{\n  \"a\": 1,\n  \"b\": 2\n}"),
            "{\n  \"a\": 1,\n  \"b\": 2\n}\n"
        );

        // Nested objects
        assert_eq!(
            format("{\"outer\": {\"inner\": 42}}"),
            "{\"outer\": {\"inner\": 42}}\n"
        );
    }

    #[test]
    fn mixed_structures() {
        assert_eq!(
            format("{\"array\": [1, 2, 3], \"object\": {\"nested\": true}}"),
            "{\"array\": [1, 2, 3], \"object\": {\"nested\": true}}\n"
        );

        assert_eq!(
            format("[{\"a\": 1}, {\"b\": 2}]"),
            "[{\"a\": 1}, {\"b\": 2}]\n"
        );
    }

    #[test]
    fn indentation() {
        let input = r#"{
"level1": {
"level2": {
"level3": "value"
}
}
}"#;
        let expected = r#"{
  "level1": {
    "level2": {
      "level3": "value"
    }
  }
}
"#;
        assert_eq!(format(input), expected);
    }

    #[test]
    fn comments_single_line() {
        let input = r#"{
  "key": "value" // This is a comment
}"#;
        let expected = r#"{
  "key": "value" // This is a comment
}
"#;
        assert_eq!(format(input), expected);
    }

    #[test]
    fn comments_multi_line() {
        let input = r#"{
  /* This is a
     multi-line comment */
  "key": "value"
}"#;
        let expected = r#"{
  /* This is a
     multi-line comment */
  "key": "value"
}
"#;
        assert_eq!(format(input), expected);
    }

    #[test]
    fn comments_leading() {
        let input = r#"// Leading comment
{
  "key": "value"
}"#;
        let expected = r#"// Leading comment
{
  "key": "value"
}
"#;
        assert_eq!(format(input), expected);
    }

    #[test]
    fn comments_mixed() {
        let input = r#"{
  // Comment before key
  "key1": "value1", // Trailing comment
  /* Block comment */
  "key2": "value2"
}"#;
        let expected = r#"{
  // Comment before key
  "key1": "value1", // Trailing comment
  /* Block comment */
  "key2": "value2"
}
"#;
        assert_eq!(format(input), expected);
    }

    #[test]
    fn various_json_types() {
        let input = r#"{
  "null": null,
  "boolean_true": true,
  "boolean_false": false,
  "integer": 42,
  "float": 3.14,
  "string": "hello world",
  "empty_string": "",
  "array": [],
  "object": {}
}"#;
        let expected = r#"{
  "null": null,
  "boolean_true": true,
  "boolean_false": false,
  "integer": 42,
  "float": 3.14,
  "string": "hello world",
  "empty_string": "",
  "array": [],
  "object": {}
}
"#;
        assert_eq!(format(input), expected);
    }

    #[test]
    fn whitespace_normalization() {
        // Test excessive whitespace removal
        let input = r#"{


  "key"   :    "value"   ,


  "another"  :   42


}"#;
        let expected = r#"{

  "key": "value",

  "another": 42
}
"#;
        assert_eq!(format(input), expected);
    }
}

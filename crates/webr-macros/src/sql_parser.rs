/// SQL template parser for MyBatis-style dynamic SQL tags.
///
/// Parses SQL strings containing `#{param}` bindings and dynamic tags:
/// `<if>`, `<where>`, `<set>`, `<foreach>`, `<choose>`, `<when>`, `<otherwise>`, `<trim>`.

/// A segment in the parsed SQL template.
#[derive(Debug)]
#[allow(dead_code)]
pub enum SqlSegment {
    /// Raw SQL text (between tags/params)
    Text(String),
    /// `#{name}` or `#{obj.field}` — named parameter binding
    Param(ParamRef),
    /// `<if test="param">...</if>`
    If { test: String, body: Vec<SqlSegment> },
    /// `<where>...</where>` — auto-prepends WHERE, strips leading AND/OR
    Where(Vec<SqlSegment>),
    /// `<set>...</set>` — auto-prepends SET, strips trailing commas
    Set(Vec<SqlSegment>),
    /// `<foreach collection="items" item="x" open="(" separator="," close=")">...</foreach>`
    ForEach {
        collection: String,
        item: String,
        open: Option<String>,
        separator: Option<String>,
        close: Option<String>,
        body: Vec<SqlSegment>,
    },
    /// `<choose><when test="a">...</when><otherwise>...</otherwise></choose>`
    Choose {
        whens: Vec<(String, Vec<SqlSegment>)>,
        otherwise: Option<Vec<SqlSegment>>,
    },
    /// `<trim prefix="" suffix="" prefixOverrides="" suffixOverrides="">...</trim>`
    Trim {
        prefix: Option<String>,
        suffix: Option<String>,
        prefix_overrides: Option<String>,
        suffix_overrides: Option<String>,
        body: Vec<SqlSegment>,
    },
}

/// A parameter reference: `#{name}` or `#{obj.field}`
#[derive(Debug, Clone)]
pub struct ParamRef {
    pub path: Vec<String>,
}

impl ParamRef {
    pub fn parse(raw: &str) -> Self {
        let path: Vec<String> = raw.split('.').map(|s| s.trim().to_string()).collect();
        Self { path }
    }
}

/// Parsed tag attributes with string-keyed lookup.
struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn get(&self, key: &str) -> Option<&String> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// Parse a SQL template string into segments.
pub fn parse_sql(input: &str) -> Vec<SqlSegment> {
    let mut parser = Parser::new(input);
    parser.parse_segments(false)
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn parse_segments(&mut self, inside_tag: bool) -> Vec<SqlSegment> {
        let mut segments = Vec::new();
        let mut text_buf = String::new();

        while !self.at_end() {
            let remaining = self.remaining();

            if inside_tag && remaining.starts_with("</") {
                break;
            }

            if remaining.starts_with("<if ") || remaining.starts_with("<if>")
                || remaining.starts_with("<where>") || remaining.starts_with("<where ")
                || remaining.starts_with("<set>") || remaining.starts_with("<set ")
                || remaining.starts_with("<foreach ")
                || remaining.starts_with("<choose>") || remaining.starts_with("<choose ")
                || remaining.starts_with("<trim ")
            {
                self.flush_text(&mut text_buf, &mut segments);
                let tag = self.parse_tag();
                segments.push(tag);
                continue;
            }

            if remaining.starts_with("#{") {
                self.flush_text(&mut text_buf, &mut segments);
                let param = self.parse_param();
                segments.push(SqlSegment::Param(param));
                continue;
            }

            let ch = remaining.chars().next().unwrap();
            text_buf.push(ch);
            self.pos += ch.len_utf8();
        }

        self.flush_text(&mut text_buf, &mut segments);
        segments
    }

    fn flush_text(&self, buf: &mut String, segments: &mut Vec<SqlSegment>) {
        if !buf.is_empty() {
            let text = std::mem::take(buf);
            segments.push(SqlSegment::Text(text));
        }
    }

    fn parse_param(&mut self) -> ParamRef {
        self.pos += 2; // skip '#{'
        let start = self.pos;
        while self.pos < self.input.len() && !self.remaining().starts_with('}') {
            self.pos += 1;
        }
        let raw = &self.input[start..self.pos];
        if self.pos < self.input.len() {
            self.pos += 1; // skip '}'
        }
        ParamRef::parse(raw.trim())
    }

    fn parse_tag(&mut self) -> SqlSegment {
        let remaining = self.remaining();
        if remaining.starts_with("<if") { self.parse_if_tag() }
        else if remaining.starts_with("<where") { self.parse_where_tag() }
        else if remaining.starts_with("<set") { self.parse_set_tag() }
        else if remaining.starts_with("<foreach") { self.parse_foreach_tag() }
        else if remaining.starts_with("<choose") { self.parse_choose_tag() }
        else if remaining.starts_with("<trim") { self.parse_trim_tag() }
        else {
            let ch = remaining.chars().next().unwrap();
            self.pos += ch.len_utf8();
            SqlSegment::Text(ch.to_string())
        }
    }

    fn parse_if_tag(&mut self) -> SqlSegment {
        let attrs = self.skip_to_tag_end();
        let test = attrs.get("test").cloned().unwrap_or_default();
        let body = self.parse_segments(true);
        self.expect_close_tag("if");
        SqlSegment::If { test, body }
    }

    fn parse_where_tag(&mut self) -> SqlSegment {
        let _attrs = self.skip_to_tag_end();
        let body = self.parse_segments(true);
        self.expect_close_tag("where");
        SqlSegment::Where(body)
    }

    fn parse_set_tag(&mut self) -> SqlSegment {
        let _attrs = self.skip_to_tag_end();
        let body = self.parse_segments(true);
        self.expect_close_tag("set");
        SqlSegment::Set(body)
    }

    fn parse_foreach_tag(&mut self) -> SqlSegment {
        let attrs = self.skip_to_tag_end();
        let collection = attrs.get("collection").cloned().unwrap_or_default();
        let item = attrs.get("item").cloned().unwrap_or_default();
        let open = attrs.get("open").cloned();
        let separator = attrs.get("separator").cloned();
        let close = attrs.get("close").cloned();
        let body = self.parse_segments(true);
        self.expect_close_tag("foreach");
        SqlSegment::ForEach { collection, item, open, separator, close, body }
    }

    fn parse_choose_tag(&mut self) -> SqlSegment {
        let _attrs = self.skip_to_tag_end();
        let mut whens = Vec::new();
        let mut otherwise = None;

        self.skip_whitespace();
        while !self.at_end() {
            self.skip_whitespace();
            let remaining = self.remaining();
            if remaining.starts_with("</choose>") { break; }
            if remaining.starts_with("<when") {
                let attrs = self.skip_to_tag_end();
                let test = attrs.get("test").cloned().unwrap_or_default();
                let body = self.parse_segments(true);
                self.expect_close_tag("when");
                whens.push((test, body));
            } else if remaining.starts_with("<otherwise") {
                let _attrs = self.skip_to_tag_end();
                let body = self.parse_segments(true);
                self.expect_close_tag("otherwise");
                otherwise = Some(body);
            } else {
                self.pos += 1;
            }
        }
        self.expect_close_tag("choose");
        SqlSegment::Choose { whens, otherwise }
    }

    fn parse_trim_tag(&mut self) -> SqlSegment {
        let attrs = self.skip_to_tag_end();
        let prefix = attrs.get("prefix").cloned();
        let suffix = attrs.get("suffix").cloned();
        let prefix_overrides = attrs.get("prefixOverrides").cloned();
        let suffix_overrides = attrs.get("suffixOverrides").cloned();
        let body = self.parse_segments(true);
        self.expect_close_tag("trim");
        SqlSegment::Trim { prefix, suffix, prefix_overrides, suffix_overrides, body }
    }

    fn skip_to_tag_end(&mut self) -> Attrs {
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] == b'>' {
                let tag_content = &self.input[start..self.pos];
                let attrs = parse_attrs(tag_content);
                self.pos += 1; // skip '>'
                return attrs;
            }
            self.pos += 1;
        }
        Attrs(Vec::new())
    }

    fn expect_close_tag(&mut self, tag_name: &str) {
        let close = format!("</{tag_name}>");
        self.skip_whitespace();
        if self.remaining().starts_with(&close) {
            self.pos += close.len();
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input.as_bytes()[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }
}

fn parse_attrs(tag_str: &str) -> Attrs {
    let mut pairs = Vec::new();
    let mut chars = tag_str.chars().peekable();

    // Skip tag name
    while let Some(&ch) = chars.peek() {
        if ch.is_alphabetic() || ch == '_' { chars.next(); } else { break; }
    }

    while chars.peek().is_some() {
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() { chars.next(); } else { break; }
        }

        let mut key = String::new();
        while let Some(&ch) = chars.peek() {
            if ch == '=' || ch.is_whitespace() { break; }
            key.push(ch);
            chars.next();
        }
        if key.is_empty() { break; }

        while let Some(&ch) = chars.peek() {
            if ch == '=' || ch.is_whitespace() { chars.next(); } else { break; }
        }

        let mut value = String::new();
        if let Some(&q) = chars.peek() {
            if q == '"' || q == '\'' {
                chars.next();
                while let Some(&ch) = chars.peek() {
                    if ch == q { chars.next(); break; }
                    value.push(ch);
                    chars.next();
                }
            }
        }
        if !key.is_empty() {
            pairs.push((key, value));
        }
    }
    Attrs(pairs)
}

/// Check if segments contain any dynamic tags.
pub fn is_dynamic(segments: &[SqlSegment]) -> bool {
    segments.iter().any(|seg| matches!(
        seg,
        SqlSegment::If { .. } | SqlSegment::Where(_) | SqlSegment::Set(_)
            | SqlSegment::ForEach { .. } | SqlSegment::Choose { .. } | SqlSegment::Trim { .. }
    ))
}



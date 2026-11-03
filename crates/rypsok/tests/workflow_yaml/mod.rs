//! A reader for the block-style YAML that the workflow files use (RFC 020,
//! handoff 020o): maps, sequences, block scalars (`|`, `>`), flow sequences
//! of scalars, quoted and plain scalars, comments. No dependency: a YAML
//! crate would put several packages in the lock file for a test. Anything
//! else (anchors, flow maps, multi-line plain scalars, tags) is refused with
//! the line number, so that a workflow written in a style this reader does
//! not know fails the gate instead of passing it unread.

/// A parsed node.
#[derive(Debug, Clone, PartialEq)]
pub enum Yaml {
    Scalar(String),
    Map(Vec<(String, Yaml)>),
    Seq(Vec<Yaml>),
}

impl Yaml {
    /// The value of `key` in a map.
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The node at a path of map keys.
    pub fn at(&self, path: &[&str]) -> Option<&Yaml> {
        path.iter().try_fold(self, |node, key| node.get(key))
    }

    /// The text of a scalar.
    pub fn scalar(&self) -> Option<&str> {
        match self {
            Yaml::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// The entries of a map; empty for any other node.
    pub fn entries(&self) -> &[(String, Yaml)] {
        match self {
            Yaml::Map(entries) => entries,
            _ => &[],
        }
    }

    /// The items of a sequence; empty for any other node.
    pub fn items(&self) -> &[Yaml] {
        match self {
            Yaml::Seq(items) => items,
            _ => &[],
        }
    }

    /// Every value stored under `key`, at any depth.
    pub fn find_all<'a>(&'a self, key: &str) -> Vec<&'a Yaml> {
        let mut found = Vec::new();
        self.collect(key, &mut found);
        found
    }

    fn collect<'a>(&'a self, key: &str, found: &mut Vec<&'a Yaml>) {
        match self {
            Yaml::Map(entries) => {
                for (k, v) in entries {
                    if k == key {
                        found.push(v);
                    }
                    v.collect(key, found);
                }
            }
            Yaml::Seq(items) => items.iter().for_each(|item| item.collect(key, found)),
            Yaml::Scalar(_) => {}
        }
    }
}

struct Line {
    number: usize,
    text: String,
}

impl Line {
    fn indent(&self) -> usize {
        self.text.len() - self.text.trim_start().len()
    }

    fn content(&self) -> &str {
        self.text.trim()
    }

    /// Blank, or a comment on a line of its own.
    fn is_skipped(&self) -> bool {
        let content = self.content();
        content.is_empty() || content.starts_with('#')
    }
}

struct Reader {
    lines: Vec<Line>,
    position: usize,
}

/// Parses `text`; panics with the line number on what it does not support.
pub fn parse(text: &str) -> Yaml {
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| Line {
            number: index + 1,
            text: line.trim_end().to_owned(),
        })
        .collect();
    let mut reader = Reader { lines, position: 0 };
    reader.skip();
    if reader.position >= reader.lines.len() {
        return Yaml::Map(Vec::new());
    }
    let indent = reader.lines[reader.position].indent();
    reader.block(indent)
}

fn is_sequence_item(content: &str) -> bool {
    content == "-" || content.starts_with("- ")
}

/// `text` without a trailing ` # comment` (outside quotes).
fn without_comment(text: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut previous = ' ';
    for (index, c) in text.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' && previous.is_whitespace() => return text[..index].trim_end(),
            None => {}
        }
        previous = c;
    }
    text
}

fn unquote(text: &str) -> String {
    let text = text.trim();
    for quote in ['"', '\''] {
        if text.len() >= 2 && text.starts_with(quote) && text.ends_with(quote) {
            return text[1..text.len() - 1].to_owned();
        }
    }
    text.to_owned()
}

/// Splits `key: rest` at the first colon outside quotes that is followed by
/// a space or the end of the text.
fn split_key(content: &str) -> Option<(String, &str)> {
    let mut quote: Option<char> = None;
    let chars: Vec<(usize, char)> = content.char_indices().collect();
    for (at, (index, c)) in chars.iter().enumerate() {
        match quote {
            Some(open) if *c == open => quote = None,
            Some(_) => {}
            None if *c == '"' || *c == '\'' => quote = Some(*c),
            None if *c == ':' => {
                let after = chars.get(at + 1).map(|(_, next)| *next);
                if after.is_none() || after == Some(' ') {
                    return Some((unquote(&content[..*index]), content[index + 1..].trim()));
                }
            }
            None => {}
        }
    }
    None
}

fn flow_sequence(text: &str) -> Yaml {
    let inner = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or_else(|| panic!("unsupported flow sequence: {text}"));
    let mut items = Vec::new();
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (index, c) in inner.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == ',' => {
                items.push(Yaml::Scalar(unquote(&inner[start..index])));
                start = index + 1;
            }
            None => {}
        }
    }
    if !inner[start..].trim().is_empty() {
        items.push(Yaml::Scalar(unquote(&inner[start..])));
    }
    Yaml::Seq(items)
}

impl Reader {
    /// Moves past blank lines and comment lines.
    fn skip(&mut self) {
        while self
            .lines
            .get(self.position)
            .is_some_and(|line| line.is_skipped())
        {
            self.position += 1;
        }
    }

    fn fail(&self, what: &str) -> ! {
        let line = &self.lines[self.position];
        panic!(
            "workflow YAML line {}: {what}: `{}`",
            line.number,
            line.content()
        );
    }

    /// A block (map or sequence) whose lines are indented by `indent`.
    fn block(&mut self, indent: usize) -> Yaml {
        let first = self.lines[self.position].content().to_owned();
        if is_sequence_item(&first) {
            self.sequence(indent)
        } else {
            self.map(indent)
        }
    }

    fn sequence(&mut self, indent: usize) -> Yaml {
        let mut items = Vec::new();
        loop {
            self.skip();
            let Some(line) = self.lines.get(self.position) else {
                break;
            };
            if line.indent() != indent || !is_sequence_item(line.content()) {
                break;
            }
            let content = without_comment(line.content()).to_owned();
            let rest = content[1..].trim_start().to_owned();
            if rest.is_empty() {
                self.position += 1;
                self.skip();
                let child = self.lines[self.position].indent();
                items.push(self.block(child));
                continue;
            }
            let child_indent = indent + (content.len() - rest.len());
            if split_key(&rest).is_some() || is_sequence_item(&rest) {
                // The item is a map (or a sequence) that starts on the dash
                // line: read that line as if it were indented to its text.
                self.lines[self.position].text = format!("{}{rest}", " ".repeat(child_indent));
                items.push(self.block(child_indent));
            } else {
                self.position += 1;
                items.push(self.value(indent, &rest));
            }
        }
        Yaml::Seq(items)
    }

    fn map(&mut self, indent: usize) -> Yaml {
        let mut entries = Vec::new();
        loop {
            self.skip();
            let Some(line) = self.lines.get(self.position) else {
                break;
            };
            if line.indent() < indent {
                break;
            }
            if line.indent() > indent || is_sequence_item(line.content()) {
                self.fail("unexpected indentation or sequence item in a map");
            }
            let content = without_comment(line.content()).to_owned();
            let Some((key, rest)) = split_key(&content) else {
                self.fail("a map entry needs `key:`");
            };
            self.position += 1;
            let value = if rest.is_empty() {
                self.skip();
                match self.lines.get(self.position) {
                    Some(next)
                        if next.indent() > indent
                            || (next.indent() == indent && is_sequence_item(next.content())) =>
                    {
                        let child = next.indent();
                        self.block(child)
                    }
                    _ => Yaml::Scalar(String::new()),
                }
            } else {
                self.value(indent, rest)
            };
            entries.push((key, value));
        }
        Yaml::Map(entries)
    }

    /// The value written after `key:` or `- ` on one line; a block scalar
    /// takes the following lines that are indented more than `indent`.
    fn value(&mut self, indent: usize, rest: &str) -> Yaml {
        if rest.starts_with('|') || rest.starts_with('>') {
            let mut body: Vec<String> = Vec::new();
            let mut strip: Option<usize> = None;
            while let Some(line) = self.lines.get(self.position) {
                let blank = line.content().is_empty();
                if !blank && line.indent() <= indent {
                    break;
                }
                let width = *strip.get_or_insert_with(|| line.indent());
                body.push(if blank {
                    String::new()
                } else {
                    line.text[width.min(line.text.len())..].to_owned()
                });
                self.position += 1;
            }
            return Yaml::Scalar(body.join("\n").trim_end().to_owned());
        }
        if rest.starts_with('[') {
            return flow_sequence(rest);
        }
        if rest.starts_with('{') || rest.starts_with('&') || rest.starts_with('*') {
            panic!("workflow YAML: unsupported value `{rest}`");
        }
        Yaml::Scalar(unquote(rest))
    }
}

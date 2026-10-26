//! Markdown rendering of an extracted subtree (RFC 012; REQ-FETCH-004):
//! a flat walk of the bounded tree with a dozen rules, no crate, no
//! recursion. Links keep absolute `http(s)` targets of bounded length;
//! images contribute their alternative text only.

use std::collections::HashSet;

use super::dom::{Dom, Handle, Kind};

/// What the renderer needs besides the tree.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// LIM-F11: subtrees deeper than this below the root are skipped.
    pub max_depth: usize,
    /// LIM-F9: a link target longer than this is rendered as text.
    pub max_url_chars: usize,
}

/// The rendering and whether the depth bound cut anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The Markdown.
    pub markdown: String,
    /// Whether a subtree deeper than the bound was skipped.
    pub depth_truncated: bool,
}

const BLOCKS: &[&str] = &[
    "p",
    "div",
    "section",
    "article",
    "main",
    "body",
    "html",
    "header",
    "footer",
    "address",
    "figure",
    "figcaption",
    "details",
    "summary",
    "dl",
    "dt",
    "dd",
    "center",
    "fieldset",
];
const HEADINGS: &[&str] = &["h1", "h2", "h3", "h4", "h5", "h6"];

enum Event {
    Enter(Handle, usize),
    Exit(Handle),
}

#[derive(Default)]
struct Table {
    rows: Vec<Vec<String>>,
    current_row: Option<Vec<String>>,
    current_cell: Option<String>,
    header_rows: usize,
}

/// The output and the state of the walk. Lines are written through
/// `begin_text` and `raw_line`, which own the blank-line discipline: a
/// block boundary asks for one blank line, and at most one is written.
#[derive(Default)]
struct Writer {
    out: String,
    /// Open list contexts: `None` for unordered, `Some(n)` for the next
    /// ordered item number.
    lists: Vec<Option<u32>>,
    blockquote: usize,
    pre: usize,
    /// The cursor is at the start of a line.
    at_line_start: bool,
    /// A blank line must precede the next line.
    pending_blank: bool,
    /// Nothing has been written since a container started: no blank line
    /// is due before its first line.
    fresh: bool,
    table: Option<Table>,
    /// Open link targets, innermost last, with the text collected so far.
    links: Vec<(Option<String>, String)>,
    emphasis: Vec<&'static str>,
}

impl Writer {
    fn new() -> Self {
        Writer {
            at_line_start: true,
            fresh: true,
            ..Default::default()
        }
    }

    fn prefix(&self) -> String {
        "> ".repeat(self.blockquote)
    }

    /// Ends the current line, dropping trailing spaces that are not a hard
    /// break.
    fn end_line(&mut self) {
        if !self.at_line_start {
            if !self.out.ends_with("  ") {
                while self.out.ends_with(' ') {
                    self.out.pop();
                }
            }
            self.out.push('\n');
            self.at_line_start = true;
        }
    }

    fn block_break(&mut self) {
        self.end_line();
        self.pending_blank = true;
    }

    /// Writes the blank line a block boundary asked for, once.
    fn flush_blank(&mut self) {
        if self.pending_blank && !self.fresh && !self.out.is_empty() {
            let blank = format!("{}\n", self.prefix().trim_end());
            if !self.out.ends_with(&format!("\n{blank}")) {
                self.out.push_str(&blank);
            }
        }
        self.pending_blank = false;
    }

    /// Starts a line of text: the blank line if due, the quote prefix and
    /// the list indentation.
    fn begin_text(&mut self) {
        if !self.at_line_start {
            return;
        }
        self.flush_blank();
        self.out.push_str(&self.prefix());
        if self.pre == 0 && !self.lists.is_empty() {
            self.out.push_str(&"  ".repeat(self.lists.len()));
        }
        self.at_line_start = false;
        self.fresh = false;
    }

    /// A line of its own (a fence, a rule, a table row).
    fn raw_line(&mut self, line: &str) {
        self.end_line();
        self.flush_blank();
        self.out.push_str(&self.prefix());
        self.out.push_str(line);
        self.out.push('\n');
        self.fresh = false;
    }

    /// Text in the flow: into the table cell or the link text when one is
    /// open, else onto the line.
    fn text(&mut self, text: &str) {
        if let Some(cell) = self.table.as_mut().and_then(|t| t.current_cell.as_mut()) {
            cell.push_str(text);
            return;
        }
        if let Some((_, collected)) = self.links.last_mut() {
            collected.push_str(text);
        }
        let text = if self.at_line_start {
            text.trim_start()
        } else {
            text
        };
        if text.is_empty() {
            return;
        }
        self.begin_text();
        self.out.push_str(text);
    }

    /// A marker written as is, never collected into link text.
    fn marker(&mut self, text: &str) {
        if let Some(cell) = self.table.as_mut().and_then(|t| t.current_cell.as_mut()) {
            cell.push_str(text);
            return;
        }
        self.begin_text();
        self.out.push_str(text);
    }
}

/// Escapes what would start Markdown syntax inside text: emphasis, code
/// and link delimiters anywhere; heading, quote and list markers at the
/// start of a line. Intraword underscores are literal in CommonMark and
/// stay readable in identifiers.
fn escape(text: &str, at_line_start: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut first = at_line_start;
    for c in text.chars() {
        match c {
            '*' | '`' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '#' | '>' | '-' | '+' if first => {
                out.push('\\');
                out.push(c);
            }
            ' ' if first => {
                continue;
            }
            _ => out.push(c),
        }
        first = false;
    }
    out
}

/// Collapses runs of whitespace into one space, as a browser renders
/// inline text; whitespace-only text is one space.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            space = true;
        } else {
            if space {
                out.push(' ');
            }
            space = false;
            out.push(c);
        }
    }
    if space {
        out.push(' ');
    }
    out
}

/// Renders the subtree under `root`, skipping `excluded` nodes.
pub fn render(dom: &Dom, root: Handle, excluded: &HashSet<Handle>, options: Options) -> Rendered {
    let mut w = Writer::new();
    let mut depth_truncated = false;
    let mut events: Vec<Event> = vec![Event::Enter(root, 0)];
    while let Some(event) = events.pop() {
        match event {
            Event::Enter(h, depth) => {
                if excluded.contains(&h) {
                    continue;
                }
                if depth > options.max_depth {
                    depth_truncated = true;
                    continue;
                }
                match &dom.node(h).kind {
                    Kind::Text(text) => render_text(&mut w, text),
                    Kind::Element { .. } => {
                        enter(&mut w, dom, h, options);
                        events.push(Event::Exit(h));
                        for &child in dom.children(h).iter().rev() {
                            events.push(Event::Enter(child, depth + 1));
                        }
                    }
                    Kind::Document => {
                        for &child in dom.children(h).iter().rev() {
                            events.push(Event::Enter(child, depth + 1));
                        }
                    }
                    Kind::Comment | Kind::Other => {}
                }
            }
            Event::Exit(h) => exit(&mut w, dom, h),
        }
    }
    w.end_line();
    let markdown = w.out.trim_matches('\n').to_owned();
    Rendered {
        markdown,
        depth_truncated,
    }
}

fn render_text(w: &mut Writer, text: &str) {
    if w.pre > 0 {
        if let Some(cell) = w.table.as_mut().and_then(|t| t.current_cell.as_mut()) {
            cell.push_str(text);
            return;
        }
        // Verbatim: every line of the block keeps its indentation.
        let mut lines = text.split('\n').peekable();
        while let Some(line) = lines.next() {
            if !line.is_empty() {
                w.begin_text();
                w.out.push_str(line);
            }
            if lines.peek().is_some() {
                w.begin_text();
                w.out.push('\n');
                w.at_line_start = true;
            }
        }
        return;
    }
    let collapsed = collapse(text);
    if collapsed.is_empty() {
        return;
    }
    let escaped = escape(&collapsed, w.at_line_start);
    w.text(&escaped);
}

fn enter(w: &mut Writer, dom: &Dom, h: Handle, options: Options) {
    let Some(name) = dom.local_name(h) else {
        return;
    };
    match name {
        n if HEADINGS.contains(&n) => {
            w.block_break();
            let level = n[1..].parse::<usize>().unwrap_or(1);
            w.marker(&format!("{} ", "#".repeat(level)));
        }
        "ul" | "ol" => {
            if w.lists.is_empty() {
                w.block_break();
            } else {
                w.end_line();
            }
            let start = (name == "ol").then(|| {
                dom.attr(h, "start")
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(1)
            });
            w.lists.push(start);
        }
        "li" => {
            w.end_line();
            w.flush_blank();
            let depth = w.lists.len().saturating_sub(1);
            let marker = match w.lists.last_mut() {
                Some(Some(n)) => {
                    let m = format!("{n}. ");
                    *n += 1;
                    m
                }
                _ => "- ".to_owned(),
            };
            let prefix = w.prefix();
            w.out.push_str(&prefix);
            w.out.push_str(&"  ".repeat(depth));
            w.out.push_str(&marker);
            w.at_line_start = false;
            w.fresh = false;
        }
        "blockquote" => {
            w.block_break();
            w.flush_blank();
            w.blockquote += 1;
            w.fresh = true;
        }
        "pre" => {
            w.block_break();
            w.raw_line("```");
            w.pre += 1;
        }
        "code" if w.pre == 0 => w.marker("`"),
        "strong" | "b" => {
            w.marker("**");
            w.emphasis.push("**");
        }
        "em" | "i" => {
            w.marker("*");
            w.emphasis.push("*");
        }
        "br" => {
            if w.table.as_ref().is_some_and(|t| t.current_cell.is_some()) {
                w.text(" ");
            } else if !w.at_line_start {
                while w.out.ends_with(' ') {
                    w.out.pop();
                }
                w.out.push_str("  \n");
                w.at_line_start = true;
            }
        }
        "hr" => {
            w.block_break();
            w.raw_line("---");
            w.pending_blank = true;
        }
        "img" => {
            if let Some(alt) = dom.attr(h, "alt") {
                let alt = collapse(alt);
                if !alt.trim().is_empty() {
                    w.text(&escape(&alt, w.at_line_start));
                }
            }
        }
        "a" => {
            let target = dom.attr(h, "href").and_then(|href| {
                let href = href.trim();
                let ok = (href.starts_with("https://") || href.starts_with("http://"))
                    && href.chars().count() <= options.max_url_chars
                    && !href
                        .chars()
                        .any(|c| c.is_whitespace() || c == ')' || c == '(');
                ok.then(|| href.to_owned())
            });
            if w.table.is_none() && target.is_some() {
                w.marker("[");
            }
            w.links.push((target, String::new()));
        }
        "table" => {
            w.block_break();
            w.table = Some(Table::default());
        }
        "tr" => {
            if let Some(t) = w.table.as_mut() {
                t.current_row = Some(Vec::new());
            }
        }
        "td" | "th" => {
            if let Some(t) = w.table.as_mut() {
                if t.current_row.is_none() {
                    t.current_row = Some(Vec::new());
                }
                t.current_cell = Some(String::new());
                if name == "th" && t.rows.is_empty() {
                    t.header_rows = 1;
                }
            }
        }
        n if BLOCKS.contains(&n) => w.block_break(),
        _ => {}
    }
}

fn exit(w: &mut Writer, dom: &Dom, h: Handle) {
    let Some(name) = dom.local_name(h) else {
        return;
    };
    match name {
        n if HEADINGS.contains(&n) => w.block_break(),
        "ul" | "ol" => {
            w.lists.pop();
            if w.lists.is_empty() {
                w.block_break();
            } else {
                w.end_line();
            }
        }
        "li" => w.end_line(),
        "blockquote" => {
            w.end_line();
            w.blockquote = w.blockquote.saturating_sub(1);
            w.pending_blank = true;
        }
        "pre" => {
            w.pre = w.pre.saturating_sub(1);
            w.raw_line("```");
            w.pending_blank = true;
        }
        "code" if w.pre == 0 => w.marker("`"),
        "strong" | "b" | "em" | "i" => {
            if let Some(marker) = w.emphasis.pop() {
                w.marker(marker);
            }
        }
        "a" => {
            let Some((target, text)) = w.links.pop() else {
                return;
            };
            if let Some(cell) = w.table.as_mut().and_then(|t| t.current_cell.as_mut()) {
                if let Some(target) = target {
                    cell.push_str(&format!(" ({target})"));
                }
            } else if let Some(target) = target {
                if text.trim().is_empty() {
                    // Nothing to label the link with: the target is the text.
                    if w.out.ends_with('[') {
                        w.out.pop();
                    }
                    w.marker(&format!("<{target}>"));
                } else {
                    w.marker(&format!("]({target})"));
                }
            }
        }
        "td" | "th" => {
            if let Some(t) = w.table.as_mut()
                && let Some(cell) = t.current_cell.take()
                && let Some(row) = t.current_row.as_mut()
            {
                row.push(collapse(&cell).trim().to_owned());
            }
        }
        "tr" => {
            if let Some(t) = w.table.as_mut()
                && let Some(row) = t.current_row.take()
                && !row.is_empty()
            {
                t.rows.push(row);
            }
        }
        "table" => {
            if let Some(table) = w.table.take() {
                write_table(w, table);
            }
        }
        n if BLOCKS.contains(&n) => w.block_break(),
        _ => {}
    }
}

fn write_table(w: &mut Writer, table: Table) {
    if table.rows.is_empty() {
        return;
    }
    let width = table.rows[0].len();
    let uniform = table.rows.iter().all(|r| r.len() == width);
    w.block_break();
    if uniform && width > 0 {
        let header_rows = table.header_rows.max(1);
        for (i, row) in table.rows.iter().enumerate() {
            let cells: Vec<String> = row.iter().map(|c| c.replace('|', "\\|")).collect();
            w.raw_line(&format!("| {} |", cells.join(" | ")));
            if i + 1 == header_rows {
                w.raw_line(&format!("|{}|", vec![" --- "; width].join("|")));
            }
        }
    } else {
        for row in &table.rows {
            w.raw_line(&row.join(" · "));
        }
    }
    w.pending_blank = true;
}

#[cfg(test)]
mod tests;

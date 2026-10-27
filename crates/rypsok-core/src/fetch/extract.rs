//! Main-content extraction (RFC 012; REQ-FETCH-002, REQ-PAGE-002,
//! REQ-CTX-005): non-rendered content and site chrome are excluded, one
//! candidate element is chosen, and its subtree is what the renderer sees.
//! Every traversal is an explicit stack bounded by LIM-F11 in depth.

use std::collections::HashSet;

use crate::html::dom::{DOCUMENT, Dom, Handle, Kind};

/// Elements a browser never renders, and `head` once the title is taken
/// (REQ-CTX-005, REQ-FETCH-009).
const NON_RENDERED: &[&str] = &["script", "style", "template", "noscript", "head"];

/// Elements that are site chrome rather than content.
const CHROME_ELEMENTS: &[&str] = &["nav", "header", "footer", "aside", "form"];

/// `role` values that mark chrome.
const CHROME_ROLES: &[&str] = &[
    "navigation",
    "banner",
    "contentinfo",
    "complementary",
    "search",
];

/// `id` or `class` tokens that mark chrome: a bare word matches the
/// token or the token with a dashed suffix (`nav`, `nav-bar`); a trailing
/// `*` matches any continuation (`advert*`: advertisement, advertising).
/// A reviewable constant; the corpus decides it (RFC 012).
pub const CHROME_TOKENS: &[&str] = &[
    "nav",
    "navbar",
    "menu",
    "sidebar",
    "footer",
    "header",
    "breadcrumb",
    "comment",
    "comments",
    "share",
    "social",
    "cookie",
    "consent",
    "banner",
    "advert*",
    "ad-",
    "ads",
    "promo",
    "related",
    "newsletter",
    "popup",
    "modal",
];

/// Children that make an element look like prose when scoring.
const PROSE_CHILDREN: &[&str] = &[
    "p",
    "li",
    "pre",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
];

/// Elements that may be a scored candidate.
const CANDIDATES: &[&str] = &["div", "section", "td", "body"];

/// Below this many characters a candidate is too small to be the page's
/// content when the body has more (RFC 012).
const TINY_CANDIDATE_CHARS: usize = 200;

/// What extraction decided: the root to render, the nodes to skip, and
/// whether the depth bound cut anything.
#[derive(Debug, Clone)]
pub struct Extraction {
    /// The raw title: `<title>`, else the candidate's first `h1`, else empty.
    pub title: String,
    /// The element whose subtree is the content.
    pub root: Handle,
    /// Nodes excluded as non-rendered or chrome, with their subtrees.
    pub excluded: HashSet<Handle>,
    /// Whether a subtree deeper than the bound was skipped.
    pub depth_truncated: bool,
}

/// Runs the extraction over a parsed tree with the depth bound LIM-F11.
pub fn extract(dom: &Dom, max_depth: u32) -> Extraction {
    let max_depth = usize::try_from(max_depth).unwrap_or(usize::MAX).max(1);
    let title_element = dom.first("title");
    let title = title_element
        .map(|t| {
            text_of(dom, t, &HashSet::new(), max_depth)
                .0
                .trim()
                .to_owned()
        })
        .unwrap_or_default();

    let mut excluded = HashSet::new();
    for (h, _) in dom.elements() {
        if is_non_rendered(dom, h) {
            excluded.insert(h);
        }
    }
    let root = candidate(dom, &excluded, max_depth);
    let ancestors = ancestors_of(dom, root);
    for (h, _) in dom.elements() {
        if !ancestors.contains(&h) && h != root && is_chrome(dom, h) {
            excluded.insert(h);
        }
    }
    let (_, depth_truncated) = text_of(dom, root, &excluded, max_depth);
    let title = if title.is_empty() {
        first_heading(dom, root, &excluded, max_depth)
            .map(|h| h.trim().to_owned())
            .unwrap_or_default()
    } else {
        title
    };
    Extraction {
        title,
        root,
        excluded,
        depth_truncated,
    }
}

fn is_non_rendered(dom: &Dom, h: Handle) -> bool {
    let Some(name) = dom.local_name(h) else {
        return false;
    };
    if NON_RENDERED.contains(&name) {
        return true;
    }
    if dom.attr(h, "hidden").is_some() {
        return true;
    }
    if dom
        .attr(h, "aria-hidden")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    {
        return true;
    }
    dom.attr(h, "style").is_some_and(hidden_by_style)
}

/// Whether an inline style hides the element: `display:none` or
/// `visibility:hidden`, whitespace-insensitive.
fn hidden_by_style(style: &str) -> bool {
    let compact: String = style
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    compact
        .split(';')
        .any(|d| d == "display:none" || d == "visibility:hidden")
}

fn is_chrome(dom: &Dom, h: Handle) -> bool {
    let Some(name) = dom.local_name(h) else {
        return false;
    };
    if CHROME_ELEMENTS.contains(&name) {
        return true;
    }
    if dom
        .attr(h, "role")
        .is_some_and(|r| CHROME_ROLES.contains(&r.trim().to_ascii_lowercase().as_str()))
    {
        return true;
    }
    let tokens = |value: &str| {
        value
            .split(|c: char| c.is_whitespace() || c == '_')
            .map(|t| t.to_ascii_lowercase())
            .collect::<Vec<_>>()
    };
    let mut all: Vec<String> = Vec::new();
    if let Some(id) = dom.attr(h, "id") {
        all.extend(tokens(id));
    }
    if let Some(class) = dom.attr(h, "class") {
        all.extend(tokens(class));
    }
    all.iter().any(|token| {
        CHROME_TOKENS.iter().any(|chrome| {
            if let Some(prefix) = chrome.strip_suffix('*') {
                token.starts_with(prefix)
            } else if let Some(prefix) = chrome.strip_suffix('-') {
                token.len() > prefix.len() + 1 && token.starts_with(&format!("{prefix}-"))
            } else {
                token == chrome || token.starts_with(&format!("{chrome}-"))
            }
        })
    })
}

/// The candidate: the first of `main`, `article`, `[role=main]` in
/// document order; else the best-scored container; else the body or the
/// document.
fn candidate(dom: &Dom, excluded: &HashSet<Handle>, max_depth: usize) -> Handle {
    let elements = dom.elements();
    let usable = |h: Handle| !excluded.contains(&h) && !is_chrome(dom, h);
    for name in ["main", "article"] {
        if let Some(&(h, _)) = elements
            .iter()
            .find(|&&(h, _)| dom.local_name(h) == Some(name) && usable(h))
        {
            return fallback_if_tiny(dom, h, excluded, max_depth);
        }
    }
    if let Some(&(h, _)) = elements.iter().find(|&&(h, _)| {
        usable(h)
            && dom
                .attr(h, "role")
                .is_some_and(|r| r.trim().eq_ignore_ascii_case("main"))
    }) {
        return fallback_if_tiny(dom, h, excluded, max_depth);
    }
    let mut best: Option<(i64, usize, Handle)> = None;
    for &(h, depth) in &elements {
        let Some(name) = dom.local_name(h) else {
            continue;
        };
        if !CANDIDATES.contains(&name) || !usable(h) {
            continue;
        }
        let score = score(dom, h, excluded, max_depth);
        if score <= 0 {
            continue;
        }
        let better = match best {
            None => true,
            Some((best_score, best_depth, _)) => {
                score > best_score || (score == best_score && depth < best_depth)
            }
        };
        if better {
            best = Some((score, depth, h));
        }
    }
    let chosen = best
        .map(|(_, _, h)| h)
        .or_else(|| dom.first("body"))
        .unwrap_or(DOCUMENT);
    fallback_if_tiny(dom, chosen, excluded, max_depth)
}

fn fallback_if_tiny(
    dom: &Dom,
    chosen: Handle,
    excluded: &HashSet<Handle>,
    max_depth: usize,
) -> Handle {
    let Some(body) = dom.first("body") else {
        return chosen;
    };
    if chosen == body {
        return chosen;
    }
    let chosen_len = text_of(dom, chosen, excluded, max_depth)
        .0
        .trim()
        .chars()
        .count();
    if chosen_len >= TINY_CANDIDATE_CHARS {
        return chosen;
    }
    let body_len = text_of(dom, body, excluded, max_depth)
        .0
        .trim()
        .chars()
        .count();
    if body_len > chosen_len { body } else { chosen }
}

/// The score of a container: the text of its prose children minus their
/// link text, halved when the container's link density exceeds one half.
fn score(dom: &Dom, h: Handle, excluded: &HashSet<Handle>, max_depth: usize) -> i64 {
    let mut total: i64 = 0;
    for &child in dom.children(h) {
        if excluded.contains(&child) {
            continue;
        }
        if dom
            .local_name(child)
            .is_some_and(|n| PROSE_CHILDREN.contains(&n))
        {
            let (text, _) = text_of(dom, child, excluded, max_depth);
            let links = link_text_len(dom, child, excluded, max_depth);
            total += text.trim().chars().count() as i64 - links as i64;
        }
    }
    let (all, _) = text_of(dom, h, excluded, max_depth);
    let all_len = all.trim().chars().count();
    if all_len > 0 && link_text_len(dom, h, excluded, max_depth) * 2 > all_len {
        total /= 2;
    }
    total
}

fn ancestors_of(dom: &Dom, h: Handle) -> HashSet<Handle> {
    let mut set = HashSet::new();
    let mut current = h;
    while let Some(parent) = dom.node(current).parent {
        set.insert(parent);
        current = parent;
    }
    set
}

/// The text under a node, skipping excluded subtrees and anything deeper
/// than `max_depth` below the node; returns whether the depth bound cut.
pub fn text_of(
    dom: &Dom,
    root: Handle,
    excluded: &HashSet<Handle>,
    max_depth: usize,
) -> (String, bool) {
    let mut out = String::new();
    let mut truncated = false;
    let mut stack: Vec<(Handle, usize)> = vec![(root, 0)];
    while let Some((h, depth)) = stack.pop() {
        if excluded.contains(&h) {
            continue;
        }
        if depth > max_depth {
            truncated = true;
            continue;
        }
        match &dom.node(h).kind {
            Kind::Text(text) => {
                out.push_str(text);
                out.push(' ');
            }
            Kind::Element { .. } | Kind::Document => {
                for &child in dom.children(h).iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
            Kind::Comment | Kind::Other => {}
        }
    }
    (out, truncated)
}

fn link_text_len(dom: &Dom, root: Handle, excluded: &HashSet<Handle>, max_depth: usize) -> usize {
    let mut total = 0;
    let mut stack: Vec<(Handle, usize)> = vec![(root, 0)];
    while let Some((h, depth)) = stack.pop() {
        if excluded.contains(&h) || depth > max_depth {
            continue;
        }
        if dom.local_name(h) == Some("a") {
            total += text_of(dom, h, excluded, max_depth.saturating_sub(depth))
                .0
                .trim()
                .chars()
                .count();
            continue;
        }
        for &child in dom.children(h).iter().rev() {
            stack.push((child, depth + 1));
        }
    }
    total
}

fn first_heading(
    dom: &Dom,
    root: Handle,
    excluded: &HashSet<Handle>,
    max_depth: usize,
) -> Option<String> {
    let mut stack: Vec<(Handle, usize)> = vec![(root, 0)];
    while let Some((h, depth)) = stack.pop() {
        if excluded.contains(&h) || depth > max_depth {
            continue;
        }
        if dom.local_name(h) == Some("h1") {
            return Some(text_of(dom, h, excluded, max_depth).0);
        }
        for &child in dom.children(h).iter().rev() {
            stack.push((child, depth + 1));
        }
    }
    None
}

#[cfg(test)]
mod tests;

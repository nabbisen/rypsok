//! Result extraction for scraping drivers (RFC 015; REQ-ENG-015): a
//! driver declares the **shape** of its provider's page, a few selectors
//! over tag name, class token and attribute, and this module walks the
//! bounded DOM of [`crate::html::dom`] iteratively to find the results,
//! the provider's "no results" marker or a challenge page. No CSS engine,
//! no recursion, raw text out (REQ-ENG-012).

use crate::engine::RawResult;
use crate::html::dom::{DOCUMENT, Dom, Handle, Kind};

/// An element selector: every given part must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selector {
    /// The local tag name, when it matters.
    pub tag: Option<&'static str>,
    /// A class token the element must carry.
    pub class: Option<&'static str>,
    /// An attribute with exactly this value.
    pub attr: Option<(&'static str, &'static str)>,
}

impl Selector {
    /// A tag with a class token.
    pub const fn class(tag: &'static str, class: &'static str) -> Self {
        Selector {
            tag: Some(tag),
            class: Some(class),
            attr: None,
        }
    }

    /// Whether `h` matches.
    pub fn matches(&self, dom: &Dom, h: Handle) -> bool {
        let Some(name) = dom.local_name(h) else {
            return false;
        };
        if self.tag.is_some_and(|tag| tag != name) {
            return false;
        }
        if let Some(class) = self.class
            && !dom
                .attr(h, "class")
                .is_some_and(|classes| classes.split_ascii_whitespace().any(|c| c == class))
        {
            return false;
        }
        if let Some((attr, value)) = self.attr
            && dom.attr(h, attr) != Some(value)
        {
            return false;
        }
        true
    }
}

/// How a page state is recognized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    /// An element matching the selector is present.
    Element(Selector),
    /// The page's text carries this substring.
    Text(&'static str),
}

/// The shape of a provider's results page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// One result.
    pub result: Selector,
    /// Within a result, the element whose `href` is the target.
    pub link: Selector,
    /// Within a result, the element whose text is the title; the link's
    /// text when absent.
    pub title: Selector,
    /// Within a result, the element whose text is the snippet; empty
    /// when absent.
    pub snippet: Selector,
    /// The provider's own "no results" marker.
    pub no_results: Marker,
    /// Any of these means a challenge or an anti-bot page.
    pub challenge: &'static [Marker],
}

/// What the page turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    /// Results in page order, targets unwrapped.
    Results(Vec<RawResult>),
    /// The provider said there is nothing.
    NoResults,
    /// A challenge or an anti-bot page (REQ-ENG-009).
    Challenge,
    /// Neither results nor the marker: the format drifted.
    Drifted,
}

/// Reads a parsed page. `unwrap` turns a link's `href` into the target
/// URL (a redirect wrapper unwrapped, REQ-ENG-001) or `None` to drop the
/// result. At most `max_results` results are returned.
pub fn read(
    dom: &Dom,
    shape: &Shape,
    max_results: usize,
    unwrap: &dyn Fn(&str) -> Option<String>,
) -> Page {
    let elements = dom.elements();
    let text = lazy_text(dom);
    for marker in shape.challenge {
        if marker_present(dom, &elements, &text, *marker) {
            return Page::Challenge;
        }
    }
    let mut results = Vec::new();
    let mut consumed_below: Option<Handle> = None;
    for &(h, _) in &elements {
        if results.len() >= max_results {
            break;
        }
        if consumed_below.is_some_and(|c| within(dom, h, c)) {
            continue;
        }
        if !shape.result.matches(dom, h) {
            continue;
        }
        consumed_below = Some(h);
        let Some(link) = find_in(dom, h, &shape.link) else {
            continue;
        };
        let Some(href) = dom.attr(link, "href") else {
            continue;
        };
        let Some(url) = unwrap(href) else {
            continue;
        };
        let title = find_in(dom, h, &shape.title)
            .map(|t| text_of(dom, t))
            .unwrap_or_else(|| text_of(dom, link));
        let snippet = find_in(dom, h, &shape.snippet)
            .map(|s| text_of(dom, s))
            .unwrap_or_default();
        // Text of a parsed document: decoded, not raw (RFC 019 D3).
        results.push(RawResult::new(title, url, snippet).decoded());
    }
    if !results.is_empty() {
        return Page::Results(results);
    }
    if marker_present(dom, &elements, &text, shape.no_results) {
        Page::NoResults
    } else {
        Page::Drifted
    }
}

fn marker_present(dom: &Dom, elements: &[(Handle, usize)], text: &str, marker: Marker) -> bool {
    match marker {
        Marker::Element(selector) => elements.iter().any(|&(h, _)| selector.matches(dom, h)),
        Marker::Text(needle) => text.contains(needle),
    }
}

/// Whether `h` is `ancestor` or below it.
fn within(dom: &Dom, mut h: Handle, ancestor: Handle) -> bool {
    loop {
        if h == ancestor {
            return true;
        }
        match dom.node(h).parent {
            Some(parent) => h = parent,
            None => return false,
        }
    }
}

/// The first descendant of `root` (itself excluded) matching `selector`,
/// in document order, by an explicit stack.
fn find_in(dom: &Dom, root: Handle, selector: &Selector) -> Option<Handle> {
    let mut stack: Vec<Handle> = dom.children(root).iter().rev().copied().collect();
    while let Some(h) = stack.pop() {
        if selector.matches(dom, h) {
            return Some(h);
        }
        stack.extend(dom.children(h).iter().rev());
    }
    None
}

/// The text below `root`, text runs joined, whitespace collapsed.
pub fn text_of(dom: &Dom, root: Handle) -> String {
    let mut out = String::new();
    let mut stack: Vec<Handle> = vec![root];
    while let Some(h) = stack.pop() {
        match &dom.node(h).kind {
            Kind::Text(text) => {
                for word in text.split_whitespace() {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(word);
                }
            }
            Kind::Element { .. } | Kind::Document => {
                stack.extend(dom.children(h).iter().rev());
            }
            Kind::Comment | Kind::Other => {}
        }
    }
    out
}

/// The whole page's text, for text markers.
fn lazy_text(dom: &Dom) -> String {
    if dom.is_empty() {
        String::new()
    } else {
        text_of(dom, DOCUMENT)
    }
}

#[cfg(test)]
mod tests;

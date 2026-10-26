//! A bounded document tree (RFC 012; REQ-PAGE-006, LIM-F10): an arena of
//! nodes filled by `html5ever` through [`BoundedSink`]. The input is fed
//! to the parser in small chunks and feeding stops as soon as the sink has
//! reached `max_nodes`, so the tree overshoots the bound by at most the
//! nodes of one chunk and the parse is recorded as truncated. No recursion
//! anywhere: every traversal is an explicit stack (THR-12).

use std::cell::{Cell, Ref, RefCell};

use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName};

/// A node handle: its index in the arena.
pub type Handle = usize;

/// The document node.
pub const DOCUMENT: Handle = 0;

/// How much input is fed to the parser between two checks of the node
/// bound; the tree can exceed the bound by the nodes of one chunk.
pub const FEED_CHUNK_BYTES: usize = 8 * 1024;

/// What a node is.
#[derive(Debug, Clone)]
pub enum Kind {
    /// The root.
    Document,
    /// An element with its attributes.
    Element {
        /// The qualified name.
        name: QualName,
        /// The attributes, as parsed.
        attrs: Vec<Attribute>,
        /// The contents of a `<template>`, never rendered.
        template_contents: Option<Handle>,
    },
    /// A text run; adjacent runs are merged.
    Text(String),
    /// A comment; never rendered.
    Comment,
    /// A processing instruction or a template's content holder; never
    /// rendered.
    Other,
}

/// One node of the arena.
#[derive(Debug, Clone)]
pub struct Node {
    /// What it is.
    pub kind: Kind,
    /// Its parent, when attached.
    pub parent: Option<Handle>,
    /// Its children, in order.
    pub children: Vec<Handle>,
}

/// The parsed document.
#[derive(Debug, Clone)]
pub struct Dom {
    nodes: Vec<Node>,
    truncated: bool,
}

impl Dom {
    fn new() -> Self {
        Dom {
            nodes: vec![Node {
                kind: Kind::Document,
                parent: None,
                children: Vec::new(),
            }],
            truncated: false,
        }
    }

    /// Whether the parse stopped at the node bound (REQ-PAGE-006).
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// How many nodes exist, the document included.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether only the document exists.
    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// A node.
    pub fn node(&self, h: Handle) -> &Node {
        &self.nodes[h]
    }

    /// The children of a node.
    pub fn children(&self, h: Handle) -> &[Handle] {
        &self.nodes[h].children
    }

    /// The lower-cased local name of an element, or none for other nodes.
    pub fn local_name(&self, h: Handle) -> Option<&str> {
        match &self.nodes[h].kind {
            Kind::Element { name, .. } => Some(&name.local),
            _ => None,
        }
    }

    /// The value of an attribute of an element, by local name.
    pub fn attr(&self, h: Handle, local: &str) -> Option<&str> {
        match &self.nodes[h].kind {
            Kind::Element { attrs, .. } => attrs
                .iter()
                .find(|a| &*a.name.local == local)
                .map(|a| &*a.value),
            _ => None,
        }
    }

    /// The text of a text node.
    pub fn text(&self, h: Handle) -> Option<&str> {
        match &self.nodes[h].kind {
            Kind::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The depth of a node under the document (the document is 0).
    pub fn depth(&self, h: Handle) -> usize {
        let mut depth = 0;
        let mut current = h;
        while let Some(parent) = self.nodes[current].parent {
            depth += 1;
            current = parent;
        }
        depth
    }

    /// Every element in document order with its depth, by an explicit
    /// stack.
    pub fn elements(&self) -> Vec<(Handle, usize)> {
        let mut out = Vec::new();
        let mut stack: Vec<(Handle, usize)> = vec![(DOCUMENT, 0)];
        while let Some((h, depth)) = stack.pop() {
            if matches!(self.nodes[h].kind, Kind::Element { .. }) {
                out.push((h, depth));
            }
            for &child in self.nodes[h].children.iter().rev() {
                stack.push((child, depth + 1));
            }
        }
        out
    }

    /// The first element with this local name in document order.
    pub fn first(&self, local: &str) -> Option<Handle> {
        self.elements()
            .into_iter()
            .map(|(h, _)| h)
            .find(|&h| self.local_name(h) == Some(local))
    }

    fn create(&mut self, kind: Kind, max_nodes: usize) -> Handle {
        if self.nodes.len() > max_nodes {
            self.truncated = true;
        }
        self.nodes.push(Node {
            kind,
            parent: None,
            children: Vec::new(),
        });
        self.nodes.len() - 1
    }

    fn detach(&mut self, h: Handle) {
        if let Some(parent) = self.nodes[h].parent.take() {
            self.nodes[parent].children.retain(|&c| c != h);
        }
    }

    fn attach(&mut self, parent: Handle, child: Handle) {
        if parent == child {
            return;
        }
        self.detach(child);
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
    }

    fn insert_before(&mut self, sibling: Handle, child: Handle) {
        let Some(parent) = self.nodes[sibling].parent else {
            return;
        };
        self.detach(child);
        let position = self.nodes[parent]
            .children
            .iter()
            .position(|&c| c == sibling)
            .unwrap_or(self.nodes[parent].children.len());
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.insert(position, child);
    }

    fn append_text(&mut self, parent: Handle, text: &str, max_nodes: usize) {
        if let Some(&last) = self.nodes[parent].children.last()
            && let Kind::Text(existing) = &mut self.nodes[last].kind
        {
            existing.push_str(text);
            return;
        }
        let h = self.create(Kind::Text(text.to_owned()), max_nodes);
        self.attach(parent, h);
    }

    fn insert_text_before(&mut self, sibling: Handle, text: &str, max_nodes: usize) {
        let Some(parent) = self.nodes[sibling].parent else {
            return;
        };
        let position = self.nodes[parent]
            .children
            .iter()
            .position(|&c| c == sibling)
            .unwrap_or(0);
        if position > 0 {
            let previous = self.nodes[parent].children[position - 1];
            if let Kind::Text(existing) = &mut self.nodes[previous].kind {
                existing.push_str(text);
                return;
            }
        }
        let h = self.create(Kind::Text(text.to_owned()), max_nodes);
        self.insert_before(sibling, h);
    }
}

/// The sink `html5ever` builds into. It counts the nodes it creates and
/// reports when the bound is reached; [`parse`] stops feeding input at the
/// next chunk boundary.
pub struct BoundedSink {
    dom: RefCell<Dom>,
    max_nodes: usize,
    quirks: Cell<QuirksMode>,
}

impl BoundedSink {
    /// A sink that reports the bound at `max_nodes` nodes.
    pub fn new(max_nodes: u32) -> Self {
        BoundedSink {
            dom: RefCell::new(Dom::new()),
            max_nodes: usize::try_from(max_nodes).unwrap_or(usize::MAX).max(1),
            quirks: Cell::new(QuirksMode::NoQuirks),
        }
    }

    /// Whether the bound has been reached.
    pub fn truncated(&self) -> bool {
        self.dom.borrow().truncated
    }

    fn create(&self, kind: Kind) -> Handle {
        self.dom.borrow_mut().create(kind, self.max_nodes)
    }
}

impl TreeSink for BoundedSink {
    type Handle = Handle;
    type Output = Dom;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Dom {
        self.dom.into_inner()
    }

    fn parse_error(&self, _msg: std::borrow::Cow<'static, str>) {}

    fn get_document(&self) -> Handle {
        DOCUMENT
    }

    fn elem_name<'a>(&'a self, target: &'a Handle) -> Ref<'a, QualName> {
        Ref::map(self.dom.borrow(), |dom| match &dom.nodes[*target].kind {
            Kind::Element { name, .. } => name,
            _ => &*NOT_AN_ELEMENT,
        })
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> Handle {
        let template_contents = flags.template.then(|| self.create(Kind::Other));
        self.create(Kind::Element {
            name,
            attrs,
            template_contents,
        })
    }

    fn create_comment(&self, _text: StrTendril) -> Handle {
        self.create(Kind::Comment)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Handle {
        self.create(Kind::Other)
    }

    fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
        let mut dom = self.dom.borrow_mut();
        match child {
            NodeOrText::AppendNode(h) => dom.attach(*parent, h),
            NodeOrText::AppendText(text) => dom.append_text(*parent, &text, self.max_nodes),
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Handle,
        prev_element: &Handle,
        child: NodeOrText<Handle>,
    ) {
        let has_parent = self.dom.borrow().nodes[*element].parent.is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
    }

    fn get_template_contents(&self, target: &Handle) -> Handle {
        let contents = match &self.dom.borrow().nodes[*target].kind {
            Kind::Element {
                template_contents: Some(contents),
                ..
            } => Some(*contents),
            _ => None,
        };
        contents.unwrap_or_else(|| self.create(Kind::Other))
    }

    fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.quirks.set(mode);
    }

    fn append_before_sibling(&self, sibling: &Handle, new_node: NodeOrText<Handle>) {
        let mut dom = self.dom.borrow_mut();
        match new_node {
            NodeOrText::AppendNode(h) => dom.insert_before(*sibling, h),
            NodeOrText::AppendText(text) => {
                dom.insert_text_before(*sibling, &text, self.max_nodes);
            }
        }
    }

    fn add_attrs_if_missing(&self, target: &Handle, new_attrs: Vec<Attribute>) {
        let mut dom = self.dom.borrow_mut();
        if let Kind::Element { attrs, .. } = &mut dom.nodes[*target].kind {
            for attr in new_attrs {
                if !attrs.iter().any(|a| a.name == attr.name) {
                    attrs.push(attr);
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &Handle) {
        self.dom.borrow_mut().detach(*target);
    }

    fn reparent_children(&self, node: &Handle, new_parent: &Handle) {
        let mut dom = self.dom.borrow_mut();
        let children = std::mem::take(&mut dom.nodes[*node].children);
        for child in children {
            dom.nodes[child].parent = None;
            dom.attach(*new_parent, child);
        }
    }
}

/// The name reported for a handle that is not an element; it matches no
/// real element.
static NOT_AN_ELEMENT: std::sync::LazyLock<QualName> = std::sync::LazyLock::new(|| {
    QualName::new(
        None,
        html5ever::ns!(),
        html5ever::LocalName::from("#not-an-element"),
    )
});

/// Parses HTML into a bounded tree. The input is fed in chunks of
/// [`FEED_CHUNK_BYTES`]; feeding stops once the sink reports the bound, so
/// the tree holds at most `max_nodes` plus one chunk's nodes. Scripting is
/// reported as disabled so that `<noscript>` content is parsed as markup
/// (and then excluded by the extractor like any other non-rendered
/// content).
pub fn parse(html: &str, max_nodes: u32) -> Dom {
    let opts = html5ever::ParseOpts {
        tree_builder: html5ever::tree_builder::TreeBuilderOpts {
            scripting_enabled: false,
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut parser = html5ever::parse_document(BoundedSink::new(max_nodes), opts);
    let mut from = 0;
    while from < html.len() {
        let mut to = (from + FEED_CHUNK_BYTES).min(html.len());
        while to < html.len() && !html.is_char_boundary(to) {
            to += 1;
        }
        parser.process(StrTendril::from_slice(&html[from..to]));
        from = to;
        if parser.tokenizer.sink.sink.truncated() {
            break;
        }
    }
    parser.finish()
}

#[cfg(test)]
mod tests;

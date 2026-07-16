// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use std::cell::RefCell;

use crate::native::{
    Element,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{ElementHandler, HandlerResult, Handlers},
    options::{LinkReferenceStyle, LinkStyle},
    text_util::{StripWhitespace, TrimDocumentWhitespace, concat_strings},
};

/// htmd keeps the pending reference-link definitions in a `thread_local!`
/// because its `ElementHandler` trait requires `Send + Sync`. The native
/// registry is per-conversion and single-threaded, so plain `RefCell` state
/// gives the same behavior without the cross-conversion leak risk.
pub(super) struct AnchorElementHandler {
    links: RefCell<Vec<String>>,
}

impl ElementHandler for AnchorElementHandler {
    fn append(&self) -> Option<String> {
        let mut links = self.links.borrow_mut();
        if links.is_empty() {
            return None;
        }
        let content_len: usize = links.iter().map(String::len).sum();
        let mut result = String::with_capacity(content_len + links.len().saturating_add(1));
        result.push_str("\n\n");
        for (index, link) in links.iter().enumerate() {
            if index > 0 {
                result.push('\n');
            }
            result.push_str(link);
        }
        result.push_str("\n\n");
        links.clear();
        Some(result)
    }

    fn handle(&self, handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
        let mut link: Option<String> = None;
        let mut title: Option<String> = None;
        for (name, value) in element.attrs {
            let name = name.local.as_ref();
            if name == "href" {
                link = Some(value.to_string())
            } else if name == "title" {
                title = Some(value.to_string());
            } else {
                // This is an attribute which can't be translated to Markdown.
                serialize_if_faithful!(handlers, element, 0);
            }
        }

        let Some(link) = link else {
            return Some(handlers.walk_children(element.node));
        };

        // Handle new lines in title
        let title = title.map(|text| process_title(&text));

        let link = escape_link_destination(&link);

        let content = handlers.walk_children(element.node).content;
        let md = match handlers.options().link_style {
            LinkStyle::Inlined => {
                self.build_inlined_anchor(&content, &link, title.as_deref(), false)
            }
            LinkStyle::InlinedPreferAutolinks => {
                self.build_inlined_anchor(&content, &link, title.as_deref(), true)
            }
            LinkStyle::Referenced => self.build_referenced_anchor(
                &content,
                link,
                title,
                &handlers.options().link_reference_style,
            ),
        };

        Some(md.into())
    }
}

impl AnchorElementHandler {
    pub(super) fn new() -> Self {
        Self {
            links: RefCell::new(Vec::new()),
        }
    }

    fn build_inlined_anchor(
        &self,
        content: &str,
        link: &str,
        title: Option<&str>,
        prefer_autolinks: bool,
    ) -> String {
        if prefer_autolinks && content == link {
            let mut result = String::with_capacity(link.len() + 2);
            result.push('<');
            result.push_str(link);
            result.push('>');
            return result;
        }

        let has_spaces_in_link = link.contains(' ');
        let (content, _) = content.strip_leading_document_whitespace();
        let (content, trailing_whitespace) = content.strip_trailing_document_whitespace();
        let title_len = title.map_or(0, |t| t.len() + 3);
        let trailing_len = trailing_whitespace.map_or(0, str::len);
        let wrapper_len = if has_spaces_in_link { 2 } else { 0 };
        let mut result = String::with_capacity(
            content.len() + link.len() + title_len + trailing_len + wrapper_len + 4,
        );
        result.push('[');
        result.push_str(content);
        result.push_str("](");
        if has_spaces_in_link {
            result.push('<');
        }
        result.push_str(link);
        if has_spaces_in_link {
            result.push('>');
        }
        if let Some(title) = title {
            result.push_str(" \"");
            result.push_str(title);
            result.push('"');
        }
        result.push(')');
        if let Some(trailing_whitespace) = trailing_whitespace {
            result.push_str(trailing_whitespace);
        }
        result
    }

    fn build_referenced_anchor(
        &self,
        content: &str,
        link: String,
        title: Option<String>,
        style: &LinkReferenceStyle,
    ) -> String {
        let title = title
            .as_deref()
            .map_or(String::new(), |t| format!(" \"{t}\""));
        let (current, append) = match style {
            LinkReferenceStyle::Full => {
                let index = self.links.borrow().len() + 1;
                (
                    concat_strings!("[", content, "][", index.to_string(), "]"),
                    concat_strings!("[", index.to_string(), "]: ", link, title),
                )
            }
            LinkReferenceStyle::Collapsed => (
                concat_strings!("[", content, "][]"),
                concat_strings!("[", content, "]: ", link, title),
            ),
            LinkReferenceStyle::Shortcut => (
                concat_strings!("[", content, "]"),
                concat_strings!("[", content, "]: ", link, title),
            ),
        };
        self.links.borrow_mut().push(append);
        current
    }
}

fn escape_link_destination(link: &str) -> String {
    if !link.contains(['(', ')']) {
        return link.to_string();
    }

    let mut escaped = String::with_capacity(link.len());
    for ch in link.chars() {
        match ch {
            '(' => escaped.push_str("\\("),
            ')' => escaped.push_str("\\)"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn process_title(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut wrote_any = false;

    for line in text.lines() {
        let line = line.trim_document_whitespace();
        if line.is_empty() {
            continue;
        }
        if wrote_any {
            result.push('\n');
        }
        for ch in line.chars() {
            if ch == '"' {
                result.push('\\');
            }
            result.push(ch);
        }
        wrote_any = true;
    }

    result
}

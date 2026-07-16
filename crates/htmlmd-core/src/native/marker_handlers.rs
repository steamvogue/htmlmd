// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native ports of the `htmd_handlers.rs` custom handlers that are active
//! with **default** `ConversionOptions`:
//!
//! - the GFM task-list checkbox handler (`semantic.task_lists` defaults to
//!   true),
//! - the `data-htmlmd-table="html"|"csv"` marker handler (the cleanup pass
//!   tags complex tables with this marker for the default
//!   `DifficultTableStrategy::HtmlFallback` / `TableHandling::CsvLike`), and
//! - the mermaid `<pre>`/`<div class="mermaid">` handlers (the default
//!   `MermaidPolicy` is `Fenced`).
//!
//! Without these, `NativeBackend` could not be byte-identical to
//! `HtmdBackend` even for `ConversionOptions::default()`. The remaining
//! custom handlers (semantic inline set, footnotes, definition lists, math,
//! alerts, wikilinks, reference links/images, `htmlmdrule` custom rules) are
//! ported in Phase B.

use ego_tree::NodeRef;
use scraper::{ElementRef, Node};

use crate::native::Element;
use crate::native::element_handler::{HandlerResult, Handlers};

// ---------------------------------------------------------------------------
// GFM task lists
// ---------------------------------------------------------------------------

pub(super) fn task_list_checkbox_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    let is_checkbox = element.attrs.iter().any(|(name, value)| {
        name.local.as_ref() == "type" && value.eq_ignore_ascii_case("checkbox")
    });
    if !is_checkbox {
        return handlers.fallback(element);
    }
    let checked = element
        .attrs
        .iter()
        .any(|(name, _)| name.local.as_ref() == "checked");
    // Raw marker text: handler output bypasses Markdown escaping.
    if checked {
        Some("[x] ".into())
    } else {
        Some("[ ] ".into())
    }
}

// ---------------------------------------------------------------------------
// Advanced table handling (data-htmlmd-table markers)
// ---------------------------------------------------------------------------

pub(super) fn table_marker_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    let marker = element
        .attrs
        .iter()
        .find(|(name, _)| name.local.as_ref() == "data-htmlmd-table")
        .map(|(_, value)| value.to_string());

    match marker.as_deref() {
        Some("html") => {
            let html = serialize_node_to_html(element.node)
                .replace(" data-htmlmd-table=\"html\"", "")
                .replace(" data-htmlmd-table=\"csv\"", "");
            if html.is_empty() {
                return Some("".into());
            }
            let fence = make_code_fence(&html, 3);
            Some(format!("\n\n{fence}html\n{html}\n{fence}\n\n").into())
        }
        Some("csv") => {
            let csv = table_to_csv(element.node);
            if csv.is_empty() {
                return Some("".into());
            }
            let fence = make_code_fence(&csv, 3);
            Some(format!("\n\n{fence}csv\n{csv}\n{fence}\n\n").into())
        }
        _ => handlers.fallback(element),
    }
}

/// The htmd path serializes the rcdom subtree with html5ever; scraper's
/// `ElementRef::html()` drives the same serializer over the ego-tree.
fn serialize_node_to_html(node: NodeRef<'_, Node>) -> String {
    ElementRef::wrap(node)
        .map(|element| element.html())
        .unwrap_or_default()
}

fn table_to_csv(node: NodeRef<'_, Node>) -> String {
    fn collect_text(node: NodeRef<'_, Node>, out: &mut String) {
        if let Node::Text(text) = node.value() {
            out.push_str(&text.text);
        }
        for child in node.children() {
            collect_text(child, out);
        }
    }

    fn escape_csv(text: &str) -> String {
        if text.contains(',') || text.contains('"') || text.contains('\n') {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_string()
        }
    }

    fn row_to_csv(node: NodeRef<'_, Node>) -> Option<String> {
        let Node::Element(element) = node.value() else {
            return None;
        };
        if element.name.local.as_ref() != "tr" {
            return None;
        }
        let cells: Vec<String> = node
            .children()
            .filter_map(|c| {
                let Node::Element(cell) = c.value() else {
                    return None;
                };
                if cell.name.local.as_ref() == "td" || cell.name.local.as_ref() == "th" {
                    let mut text = String::new();
                    collect_text(c, &mut text);
                    Some(escape_csv(text.trim()))
                } else {
                    None
                }
            })
            .collect();
        if cells.is_empty() {
            None
        } else {
            Some(cells.join(","))
        }
    }

    fn collect_rows(node: NodeRef<'_, Node>, rows: &mut Vec<String>) {
        let Node::Element(element) = node.value() else {
            for child in node.children() {
                collect_rows(child, rows);
            }
            return;
        };
        if element.name.local.as_ref() == "tr" {
            if let Some(line) = row_to_csv(node) {
                rows.push(line);
            }
            return;
        }
        for child in node.children() {
            collect_rows(child, rows);
        }
    }

    let mut rows = Vec::new();
    collect_rows(node, &mut rows);
    rows.join("\n")
}

// ---------------------------------------------------------------------------
// Mermaid / diagram handling
// ---------------------------------------------------------------------------

pub(super) fn mermaid_drop_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    if element_has_class(&element, "mermaid") {
        Some("".into())
    } else {
        handlers.fallback(element)
    }
}

pub(super) fn mermaid_fenced_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    if !element_has_class(&element, "mermaid") {
        return handlers.fallback(element);
    }
    let text = handlers
        .walk_children(element.node)
        .content
        .trim()
        .to_string();
    if text.is_empty() {
        return Some("".into());
    }
    let fence = make_code_fence(&text, 3);
    Some(format!("\n\n{fence}mermaid\n{text}\n{fence}\n\n").into())
}

fn element_has_class(element: &Element<'_>, class: &str) -> bool {
    element
        .attrs
        .iter()
        .find(|(name, _)| name.local.as_ref() == "class")
        .map(|(_, value)| value.split_whitespace().any(|c| c == class))
        .unwrap_or(false)
}

fn make_code_fence(content: &str, min_len: usize) -> String {
    let mut len = min_len;
    while content.contains(&"`".repeat(len)) {
        len += 1;
    }
    "`".repeat(len)
}

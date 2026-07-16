// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

//! The DOM walker, ported from htmd's `dom_walker.rs`.
//!
//! Mapping from `markup5ever_rcdom` to scraper's `ego_tree`:
//!
//! - `&Rc<Node>` becomes `NodeRef<'_, Node>` (`Copy`, passed by value).
//! - `node.children.borrow()` becomes `node.children()`.
//! - `NodeData::{Document, Text, Element, Comment, Doctype}` become the
//!   corresponding `scraper::Node` variants (`Fragment` is treated like
//!   `Document`).
//! - htmd's `walk_children` *mutates* the DOM to combine similar adjacent
//!   inline elements (it appends the second node's text to the first node's
//!   single text child and removes the second node). `NodeRef` trees are
//!   immutable, so `combine_children` records the merged text in a side table
//!   on `ElementHandlers` (keyed by the surviving text node's `NodeId`) and
//!   skips the absorbed siblings. The combination is deterministic and
//!   idempotent, so repeated walks of the same parent behave exactly like
//!   htmd's one-time mutation.

use ego_tree::NodeRef;
use scraper::Node;
use std::borrow::Cow;

use crate::native::element_handler::ElementHandlers;

use super::{
    options::TranslationMode,
    text_util::{
        TrimDocumentWhitespace, compress_whitespace, index_of_markdown_ordered_item_dot,
        is_markdown_atx_heading,
    },
};

pub(crate) fn walk_node(
    node: NodeRef<'_, Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    parent_tag: Option<&str>,
    trim_leading_spaces: bool,
    is_pre: bool,
) -> bool {
    let mut markdown_translated = true;
    match node.value() {
        Node::Document | Node::Fragment => {
            let _ = walk_children(node, output, handlers, true, false);
            trim_output_end(output);
        }

        Node::Text(contents) => {
            // htmd reads the (possibly merged) rcdom text contents here; we
            // consult the combined-text side table instead.
            let overrides = handlers.combined_texts.borrow();
            let text: &str = overrides
                .get(&node.id())
                .map(String::as_str)
                .unwrap_or(&contents.text);
            if is_pre {
                // Handle pre and code
                let text = if parent_tag.is_some_and(|t| t == "pre") {
                    escape_pre_text_if_needed(Cow::Borrowed(text))
                } else {
                    Cow::Borrowed(text)
                };
                output.push_str(text.as_ref());
            } else {
                let last_ends_with_space = output.ends_with(' ');
                if is_plain_text(text) {
                    let text =
                        if trim_leading_spaces || (text.starts_with(' ') && last_ends_with_space) {
                            text.trim_start_matches(' ')
                        } else {
                            text
                        };
                    if !text.is_empty() {
                        output.push_str(text);
                    }
                    return markdown_translated;
                }

                // Handle other elements or texts
                let text = escape_if_needed(Cow::Borrowed(text));
                let text = compress_whitespace(text.as_ref());

                let to_add = if trim_leading_spaces
                    || (text.chars().next().is_some_and(|ch| ch == ' ') && last_ends_with_space)
                {
                    // We can't compress spaces between two text blocks/elements, so we
                    // compress them here by trimming the leading space of current text
                    // content.
                    text.trim_start_matches(' ')
                } else {
                    text.as_ref()
                };
                if !to_add.is_empty() {
                    output.push_str(to_add);
                }
            }
        }

        Node::Element(element) => {
            // Visit this element.
            let tag = element.name.local.as_ref();
            let is_head = tag == "head";

            let res = handlers.handle(
                node,
                tag,
                &element.attrs,
                true, // Default to true, handler will update
                0,
            );

            if let Some(res) = res {
                markdown_translated = res.markdown_translated;
                if !res.content.is_empty() || !is_head {
                    append_normalized_content(output, res.content, is_pre);
                }
            }
        }

        Node::Comment(contents) => {
            if handlers.options.translation_mode == TranslationMode::Faithful {
                output.push_str("<!--");
                output.push_str(contents);
                output.push_str("-->");
            }
        }
        // rcdom never yields these to htmd's walker (its HTML parser turns
        // `<?...>` into comments); ignore them rather than panic.
        Node::Doctype(_) | Node::ProcessingInstruction(_) => {}
    }

    markdown_translated
}

fn is_plain_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    let Some(&first) = bytes.first() else {
        return true;
    };

    if matches!(first, b'=' | b'~' | b'>' | b'-' | b'+' | b'#' | b'0'..=b'9') {
        return false;
    }

    let mut previous_was_space = false;
    for &byte in bytes {
        match byte {
            b'\\' | b'*' | b'_' | b'`' | b'[' | b']' | b'<' => return false,
            b' ' => {
                if previous_was_space {
                    return false;
                }
                previous_was_space = true;
            }
            b'\t' | b'\n' | b'\r' | 0x0C | 0x0B => return false,
            _ => previous_was_space = false,
        }
    }

    true
}

pub(crate) fn walk_children(
    node: NodeRef<'_, Node>,
    output: &mut String,
    handlers: &ElementHandlers,
    is_parent_block_element: bool,
    is_pre: bool,
    // Return value: `markdown_translated`.
) -> bool {
    // Combine similar adjacent blocks (htmd mutates the DOM here; see the
    // module docs for how the side table replicates that).
    let children = combine_children(node, handlers);

    // Trim leading spaces of the first element/text in block elements (except pre/code)
    let mut trim_leading_spaces = !is_pre && is_parent_block_element;
    let tag = crate::native::node_util::get_node_tag_name(node);
    let mut markdown_translated = true;
    for child in children {
        let is_block = match child.value() {
            Node::Element(element) => is_block_element(element.name.local.as_ref()),
            _ => false,
        };

        if is_block {
            // Trim trailing spaces for the previous element
            trim_output_end_spaces(output);
        }

        let output_len = output.len();

        markdown_translated &= walk_node(child, output, handlers, tag, trim_leading_spaces, is_pre);

        if output.len() > output_len {
            // Something was appended, update the flag
            trim_leading_spaces = is_block;
        }
    }

    markdown_translated
}

/// Compute the effective child list after combining similar adjacent inline
/// elements, mirroring the mutation loop at the top of htmd's `walk_children`:
/// runs of combinable elements collapse into the first element of the run,
/// whose single text child is given the concatenated text via the side table.
fn combine_children<'a>(
    node: NodeRef<'a, Node>,
    handlers: &ElementHandlers,
) -> Vec<NodeRef<'a, Node>> {
    let children: Vec<NodeRef<'a, Node>> = node.children().collect();
    if children.len() <= 1 {
        return children;
    }

    let mut effective: Vec<NodeRef<'a, Node>> = Vec::with_capacity(children.len());
    let mut index = 0;
    while index < children.len() {
        let first = children[index];
        let mut end = index + 1;
        let mut merged: Option<String> = None;
        while end < children.len() {
            let Some(text) = can_combine(first, children[end]) else {
                break;
            };
            merged
                .get_or_insert_with(|| {
                    single_text_child(first)
                        .map(str::to_string)
                        .unwrap_or_default()
                })
                .push_str(text);
            end += 1;
        }
        if let Some(merged) = merged {
            // `can_combine` guarantees `first` has a single text child.
            if let Some(text_node) = single_text_child_node(first) {
                handlers
                    .combined_texts
                    .borrow_mut()
                    .insert(text_node.id(), merged);
            }
        }
        effective.push(first);
        index = end;
    }
    effective
}

// Determine if the two nodes are similar, and should therefore be combined. If
// so, return the text of the second node to simplify the combining process.
fn can_combine<'a>(n1: NodeRef<'a, Node>, n2: NodeRef<'a, Node>) -> Option<&'a str> {
    // To be combined, both nodes must be elements.
    let Node::Element(e1) = n1.value() else {
        return None;
    };
    let Node::Element(e2) = n2.value() else {
        return None;
    };

    // Only combine inline content; block content (for example, one paragraph
    // following another) repetition is expected and should not be combined.
    if is_block_element(e1.name.local.as_ref()) {
        return None;
    }

    // htmd requires both rcdom nodes' `template_contents` to be `None`, which
    // is never true for a parsed <template> element. scraper stores template
    // contents as regular children, so replicate the exclusion by tag name.
    if e1.name.local.as_ref() == "template" || e2.name.local.as_ref() == "template" {
        return None;
    }

    // Their children must be a single text element.
    single_text_child(n1)?;
    let text2 = single_text_child(n2)?;

    let local1 = e1.name.local.as_ref();
    let local2 = e2.name.local.as_ref();

    // Don't combine adjacent hyperlinks.
    if local1 == "a" {
        return None;
    }

    let similar_name = e1.name == e2.name
        // Treat `i` and `em` tags as the same element; likewise for `b` and
        // `strong`.
        || (local1 == "i" && local2 == "em")
        || (local1 == "em" && local2 == "i")
        || (local1 == "b" && local2 == "strong")
        || (local1 == "strong" && local2 == "b");

    // rcdom also compares `mathml_annotation_xml_integration_point`; scraper
    // does not track it, and for same-named elements parsed from the same
    // document the flags are always equal, so no check is needed.
    if similar_name && e1.attrs == e2.attrs {
        Some(text2)
    } else {
        None
    }
}

fn single_text_child_node<'a>(node: NodeRef<'a, Node>) -> Option<NodeRef<'a, Node>> {
    let mut children = node.children();
    let first = children.next()?;
    if children.next().is_some() {
        return None;
    }
    first.value().is_text().then_some(first)
}

fn single_text_child<'a>(node: NodeRef<'a, Node>) -> Option<&'a str> {
    match single_text_child_node(node)?.value() {
        Node::Text(text) => Some(&text.text),
        _ => None,
    }
}

/// Normalizes content before adding to output by:
/// 1. Collapsing excessive newlines (max 2 consecutive newlines)
/// 2. Collapsing adjacent spaces between inline elements (when not in pre context)
fn append_normalized_content(output: &mut String, mut content: String, is_pre: bool) {
    if output.is_empty() {
        output.push_str(&content);
        return;
    }

    let last_newlines = output.chars().rev().take_while(|c| *c == '\n').count();
    let content_newlines = content.chars().take_while(|c| *c == '\n').count();
    let total_newlines = last_newlines + content_newlines;

    // Collapse excessive newlines (max 2)
    if total_newlines > 2 {
        let to_remove = std::cmp::min(total_newlines - 2, content_newlines);
        content.drain(..to_remove);
    }

    // Collapse adjacent spaces between inline elements (not in pre context)
    if !is_pre
        && last_newlines == 0
        && content_newlines == 0
        && output.ends_with(' ')
        && content.chars().next().is_some_and(|c| c == ' ')
    {
        content.remove(0);
    }

    output.push_str(&content);
}

fn trim_output_end(output: &mut String) {
    let trimmed_len = output.trim_end_document_whitespace().len();
    output.truncate(trimmed_len);
}

fn trim_output_end_spaces(output: &mut String) {
    let trimmed_len = output.trim_end_matches(' ').len();
    output.truncate(trimmed_len);
}

/// Cases:
/// '\'        -> '\\'
/// '==='      -> '\==='      // h1
/// '---'      -> '\---'      // h2
/// '```'      -> '\```'       // code fence
/// '~~~'      -> '\~~~'       // code fence
/// '# Not h1' -> '\\# Not h1' // markdown heading in html
/// '1. Item'  -> '1\\. Item'  // ordered list item
/// '- Item'   -> '\\- Item'   // unordered list item
/// '+ Item'   -> '\\+ Item'   // unordered list item
/// '> Quote'  -> '\\> Quote'  // quote
fn escape_if_needed(text: Cow<'_, str>) -> Cow<'_, str> {
    let Some(first) = text.chars().next() else {
        return text;
    };

    let mut need_escape = matches!(first, '=' | '~' | '>' | '-' | '+' | '#' | '0'..='9');

    if !need_escape {
        need_escape = text
            .chars()
            .any(|c| c == '\\' || c == '*' || c == '_' || c == '`' || c == '[' || c == ']');
    }

    if !need_escape {
        return crate::native::html_escape::escape_html(text);
    }

    let mut escaped = String::new();
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '*' => escaped.push_str("\\*"),
            '_' => escaped.push_str("\\_"),
            '`' => escaped.push_str("\\`"),
            '[' => escaped.push_str("\\["),
            ']' => escaped.push_str("\\]"),
            _ => escaped.push(ch),
        }
    }

    match first {
        '=' | '~' | '>' => {
            escaped.insert(0, '\\');
        }
        '-' | '+' => {
            if escaped.chars().nth(1).is_some_and(|ch| ch == ' ') {
                escaped.insert(0, '\\');
            }
        }
        '#' => {
            if is_markdown_atx_heading(&escaped) {
                escaped.insert(0, '\\');
            }
        }
        '0'..='9' => {
            if let Some(dot_idx) = index_of_markdown_ordered_item_dot(&escaped) {
                escaped.replace_range(dot_idx..(dot_idx + 1), "\\.");
            }
        }
        _ => {}
    }

    // Perform the HTML escape after the other escapes, so that the \\
    // characters inserted here don't get escaped again.
    crate::native::html_escape::escape_html(escaped.into())
}

/// Cases:
/// '```' -> '\```' // code fence
/// '~~~' -> '\~~~' // code fence
fn escape_pre_text_if_needed(text: Cow<'_, str>) -> Cow<'_, str> {
    let Some(first) = text.chars().next() else {
        return text;
    };
    match first {
        '`' | '~' => {
            let mut escaped = String::with_capacity(text.len() + 1);
            escaped.push('\\');
            escaped.push_str(text.as_ref());
            Cow::Owned(escaped)
        }
        _ => text,
    }
}

// This is taken from the
// [CommonMark spec](https://spec.commonmark.org/0.31.2/#html-blocks).
// htmd uses a `phf` set; a `matches!` over the same tag list is behaviorally
// identical and avoids a new dependency.
pub(crate) fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "base"
            | "basefont"
            | "blockquote"
            | "body"
            | "caption"
            | "center"
            | "col"
            | "colgroup"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frame"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hr"
            | "html"
            | "iframe"
            | "legend"
            | "li"
            | "link"
            | "main"
            | "menu"
            | "menuitem"
            | "nav"
            | "noframes"
            | "ol"
            | "optgroup"
            | "option"
            | "p"
            | "param"
            | "pre"
            | "script"
            | "search"
            | "section"
            | "style"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "textarea"
            | "tfoot"
            | "th"
            | "thead"
            | "title"
            | "tr"
            | "track"
            | "ul"
    )
}

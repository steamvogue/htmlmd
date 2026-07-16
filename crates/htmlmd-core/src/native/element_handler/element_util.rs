// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    dom_walker::is_block_element,
    element_handler::{HandlerResult, Handlers},
    node_util::parent_tag_name_equals,
    options::TranslationMode,
    text_util::concat_strings,
};
use markup5ever::{QualName, local_name, ns};
use scraper::ElementRef;

// A handler for tags whose only criteria (for faithful translation) is the tag
// name of the parent.
pub(super) fn handle_or_serialize_by_parent(
    handlers: &dyn Handlers,
    // The element to check.
    element: &Element,
    // A list of allowable tag names for this element's parent.
    tag_names: &[&str],
    // The value for `markdown_translate` to pass if this tag is markdown translatable.
    markdown_translated: bool,
) -> Option<HandlerResult> {
    // In faithful mode, fall back to HTML when this element's parent tag is not
    // in `tag_names` (e.g., `<tbody>` outside `<table>`, `<td>` outside `<tr>`, etc.).
    if handlers.options().translation_mode == TranslationMode::Faithful
        && !parent_tag_name_equals(element.node, tag_names)
    {
        Some(HandlerResult {
            content: serialize_element(handlers, element),
            markdown_translated: false,
        })
    } else {
        let content = handlers.walk_children(element.node).content;
        let content = content.trim_matches('\n');
        Some(HandlerResult {
            content: concat_strings!("\n\n", content, "\n\n"),
            markdown_translated,
        })
    }
}

// Given a node (which must be an element), serialize it (transform it back
// to HTML).
//
// htmd drives html5ever's `HtmlSerializer` over rcdom here. The workspace's
// direct `html5ever` dependency (0.38, shared with htmd/rcdom) uses different
// `QualName`/tendril types than scraper's html5ever (0.39), so this port
// serializes through scraper instead: `ElementRef::html()` for the block
// case, and a hand-rolled start/end tag that mirrors `HtmlSerializer`'s
// output byte-for-byte for the inline case.
pub(crate) fn serialize_element(handlers: &dyn Handlers, element: &Element) -> String {
    // If this is a block element, then serialize it and all its children.
    // Otherwise, serialize just this element, but use the current contents in
    // the place of children. This follows the Commonmark spec: [HTML
    // blocks](https://spec.commonmark.org/0.31.2/#html-blocks) contain only
    // HTML, not Markdown, while [raw HTML
    // inlines](https://spec.commonmark.org/0.31.2/#raw-html) contain Markdown.
    if !is_block_element(element.tag) {
        // Write this element's start tag.
        let mut result = String::new();
        result.push('<');
        result.push_str(element.tag);
        for (name, value) in element.attrs {
            result.push(' ');
            push_attr_name(&mut result, name);
            result.push_str("=\"");
            push_escaped(&mut result, value, true);
            result.push('"');
        }
        result.push('>');
        // Write out the contents, without escaping them. The standard
        // serialization process escapes the contents, hence this manual
        // approach.
        result.push_str(&handlers.walk_children(element.node).content);
        // Write the end tag, if needed (html5ever omits it for void elements).
        if !is_void_element(element.tag) {
            result.push_str("</");
            result.push_str(element.tag);
            result.push('>');
        }
        result
    } else {
        let Some(element_ref) = ElementRef::wrap(element.node) else {
            // Should be unreachable: only elements are dispatched here.
            return String::new();
        };
        let s = element_ref.html();
        // We must avoid consecutive newlines in HTML blocks, since this
        // terminates the block per the CommonMark spec. Therefore, this
        // code replaces instances of two or more newlines with a single
        // newline, followed by escaped newlines. This is a hand-coded
        // version of the following regex:
        //
        // ```Rust
        // Regex::new(r#"(\r?\n\s*)(\r?\n\s*)"#).unwrap())
        //  .replace_all(&s, |caps: &Captures| {
        //      caps[1].to_string()
        //      + &(caps[2].replace("\r", "&#13;").replace("\n", "&#10;"))
        //  })
        // ```
        //
        // 1.  If the next character is an \\r or \\n, output it.
        // 2.  If the previous character was a \\r and the next
        //     character isn't a \\n, restart. Otherwise, output the
        //     \\n.
        // 3.  If the next character is whitespace but not \\n or \\r,
        //     output it then repeat this step.
        // 4.  If the next character is a \\r and the peeked following
        //     character isn't an \\n, output the \\r and restart.
        //     Otherwise, output an encoded \\r.
        // 5.  If the peeked next character is a \\n, output an encoded
        //     \\n. Otherwise, restart.
        // 6.  If the next character is whitespace but not \\n or \\r,
        //     output it then repeat this step. Otherwise, restart.
        //
        // Replace instances of two or more newlines with a newline
        // followed by escaped newlines
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            // Step 1.
            if c == '\r' || c == '\n' {
                result.push(c);

                // Step 2.
                if c == '\r' {
                    if chars.peek() == Some(&'\n') {
                        result.push(chars.next().unwrap());
                    } else {
                        continue;
                    }
                }

                // Step 3: Skip any whitespace after the newline.
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() && next != '\r' && next != '\n' {
                        result.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Step 4.
                if let Some(c) = chars.next() {
                    if c == '\r' || c == '\n' {
                        if c == '\r' {
                            if chars.peek() == Some(&'\n') {
                                chars.next();
                                result.push_str("&#13;&#10;");
                            } else {
                                // Step 6.
                                result.push('\r');
                                continue;
                            }
                        } else {
                            result.push_str("&#10;");
                        }

                        // Step 6.
                        while let Some(&next) = chars.peek() {
                            if next.is_whitespace() && next != '\r' && next != '\n' {
                                result.push(next);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                    } else {
                        result.push(c);
                    }
                }
            } else {
                result.push(c);
            }
        }
        concat_strings!("\n\n", result, "\n\n")
    }
}

/// Attribute-name serialization, mirroring html5ever's `HtmlSerializer`.
fn push_attr_name(out: &mut String, name: &QualName) {
    match name.ns {
        ns!() => (),
        ns!(xml) => out.push_str("xml:"),
        ns!(xmlns) => {
            if name.local != local_name!("xmlns") {
                out.push_str("xmlns:");
            }
        }
        ns!(xlink) => out.push_str("xlink:"),
        _ => out.push_str("unknown_namespace:"),
    }
    out.push_str(name.local.as_ref());
}

/// Text escaping, mirroring html5ever's `HtmlSerializer::write_escaped`.
fn push_escaped(out: &mut String, text: &str, attr_mode: bool) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\u{00A0}' => out.push_str("&nbsp;"),
            '"' if attr_mode => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

/// Void elements for which html5ever's serializer omits the end tag.
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "basefont"
            | "bgsound"
            | "br"
            | "col"
            | "embed"
            | "frame"
            | "hr"
            | "img"
            | "input"
            | "keygen"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

// When in faithful translation mode, return an HTML translation if this element
// has more than the allowed number of attributes.
macro_rules! serialize_if_faithful {
    (
        // The handlers to use for serialization.
        $handlers: expr,
        // The element to translate.
        $element: expr,
        // The maximum number of attributes allowed for this element. Supply
        // -1 to serialize in faithful mode, even with no attributes.
        $num_attrs_allowed: expr
    ) => {
        if $handlers.options().translation_mode
            == $crate::native::options::TranslationMode::Faithful
            && $element.attrs.len() as i64 > $num_attrs_allowed
        {
            return Some($crate::native::element_handler::HandlerResult {
                content: $crate::native::element_handler::element_util::serialize_element(
                    $handlers, &$element,
                ),
                // This was translated using HTML, not Markdown.
                markdown_translated: false,
            });
        }
    };
}

pub(crate) use serialize_if_faithful;

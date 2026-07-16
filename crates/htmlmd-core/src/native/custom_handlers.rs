// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native ports of the custom element handlers in `htmd_handlers.rs`,
//! mirroring that module section-for-section so the two stay easy to diff.
//! `build_native_converter` registers these in the same order as
//! `htmd_handlers::build_converter`, with identical profile gating, so
//! `NativeBackend` output is byte-identical to `HtmdBackend` for every
//! profile (enforced by `tests/native_parity.rs`).
//!
//! Concurrency note: `htmd_handlers.rs` shares handler state (reference-link
//! definitions) behind `Arc<Mutex<_>>` with poison recovery because htmd's
//! `ElementHandler` trait requires `Send + Sync`. The native registry is
//! built per conversion and never crosses threads, so `Rc<RefCell<_>>`
//! provides the same semantics without locking.

use std::cell::RefCell;
use std::rc::Rc;

use ego_tree::NodeRef;
use scraper::{ElementRef, Node};

use crate::native::element_handler::{ElementHandler, HandlerResult, Handlers};
use crate::native::options::TranslationMode;
use crate::native::{Element, NativeConverterBuilder};
use crate::options::{
    ConversionOptions, CustomRule, CustomRuleAction, ImageMode, LinkStyle as HtmlMdLinkStyle,
    MathOutput, MermaidPolicy, OutputProfile, ReferencePlacement,
};

// ---------------------------------------------------------------------------
// Semantic handlers
// ---------------------------------------------------------------------------

pub(super) fn add_semantic_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    let profile = options.profile;
    let gfm_plus = matches!(
        profile,
        OutputProfile::Gfm
            | OutputProfile::Extended
            | OutputProfile::Pandoc
            | OutputProfile::Obsidian
            | OutputProfile::MdxSafe
    );
    let extended = matches!(
        profile,
        OutputProfile::Extended
            | OutputProfile::Pandoc
            | OutputProfile::Obsidian
            | OutputProfile::MdxSafe
    );

    if gfm_plus {
        // Strikethrough is valid GFM.
        builder = builder.add_handler(vec!["del", "s", "strike"], strikethrough_handler);
    }

    if extended {
        builder = builder
            .add_handler(vec!["mark"], mark_handler)
            .add_handler(vec!["ins"], ins_handler)
            .add_handler(vec!["sub"], sub_handler)
            .add_handler(vec!["sup"], sup_handler)
            .add_handler(vec!["samp", "var"], code_like_handler)
            .add_handler(vec!["q"], q_handler)
            .add_handler(vec!["cite"], cite_handler)
            .add_handler(vec!["abbr", "time"], unwrap_handler)
            .add_handler(vec!["address"], address_handler);

        // <kbd> emits raw HTML; keep it out of MDX-safe output.
        if profile != OutputProfile::MdxSafe {
            builder = builder.add_handler(vec!["kbd"], kbd_handler);
        }
    }

    builder
}

fn strikethrough_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "~~", "~~")
}

fn mark_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "==", "==")
}

fn ins_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "++", "++")
}

fn sub_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "~", "~")
}

fn sup_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "^", "^")
}

fn code_like_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "`", "`")
}

fn kbd_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if faithful_with_attrs(h, &e) {
        return h.fallback(e);
    }
    let content = walk_children(h, &e);
    Some(format!("<kbd>{content}</kbd>").into())
}

fn q_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if faithful_with_attrs(h, &e) {
        return h.fallback(e);
    }
    let content = walk_children(h, &e);
    Some(format!("\"{content}\"").into())
}

fn cite_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    inline_wrap(h, e, "*", "*")
}

fn unwrap_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if faithful_with_attrs(h, &e) {
        return h.fallback(e);
    }
    Some(walk_children(h, &e).into())
}

fn address_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if faithful_with_attrs(h, &e) {
        return h.fallback(e);
    }
    let content = walk_children(h, &e).trim().to_string();
    if content.is_empty() {
        Some("".into())
    } else {
        Some(format!("\n\n{content}\n\n").into())
    }
}

// ---------------------------------------------------------------------------
// GFM task lists
// ---------------------------------------------------------------------------

pub(super) fn add_task_list_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    if !options.semantic.task_lists {
        return builder;
    }

    builder = builder.add_handler(vec!["input"], task_list_checkbox_handler);
    builder
}

fn task_list_checkbox_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    let is_checkbox = e.attrs.iter().any(|(name, value)| {
        name.local.as_ref() == "type" && value.eq_ignore_ascii_case("checkbox")
    });
    if !is_checkbox {
        return h.fallback(e);
    }
    let checked = e
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
// Footnotes
// ---------------------------------------------------------------------------

pub(super) fn add_footnote_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    if !options.semantic.footnotes {
        return builder;
    }

    builder = builder.add_handler(vec!["sup"], footnote_ref_handler);
    builder = builder.add_handler(vec!["li"], footnote_def_handler);

    builder
}

fn footnote_ref_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if let Some(label) = footnote_reference_label(e.node) {
        return Some(format!("[^{label}]").into());
    }
    h.fallback(e)
}

fn footnote_def_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    let id = e
        .attrs
        .iter()
        .find(|(name, _)| name.local.as_ref() == "id")
        .map(|(_, value)| value.to_string());
    let Some(id) = id else {
        return h.fallback(e);
    };
    let Some(label) = id.strip_prefix("fn").map(normalize_footnote_label) else {
        return h.fallback(e);
    };
    let content = footnote_definition_content(h, e.node);
    Some(format!("\n\n[^{label}]: {}\n\n", content.trim()).into())
}

fn footnote_reference_label(node: NodeRef<'_, Node>) -> Option<String> {
    let mut children = node.children();
    let child = children.next()?;
    if children.next().is_some() {
        return None;
    }
    let Node::Element(element) = child.value() else {
        return None;
    };
    if element.name.local.as_ref() != "a" {
        return None;
    }
    let href = element
        .attrs
        .iter()
        .find(|(name, _)| name.local.as_ref() == "href")
        .map(|(_, value)| value.to_string())?;
    href.strip_prefix("#fn").map(normalize_footnote_label)
}

fn footnote_definition_content(handlers: &dyn Handlers, node: NodeRef<'_, Node>) -> String {
    node.children()
        .filter_map(|child| {
            if is_backlink(child) {
                return None;
            }
            handlers.handle(child).map(|r| r.content)
        })
        .collect::<String>()
}

fn normalize_footnote_label(s: &str) -> String {
    s.trim_start_matches(['-', ':', '_']).to_string()
}

fn is_backlink(node: NodeRef<'_, Node>) -> bool {
    let Node::Element(element) = node.value() else {
        return false;
    };
    if element.name.local.as_ref() != "a" {
        return false;
    }
    element
        .attrs
        .iter()
        .any(|(name, value)| name.local.as_ref() == "href" && value.starts_with("#fnref"))
}

// ---------------------------------------------------------------------------
// Definition lists
// ---------------------------------------------------------------------------

pub(super) fn add_definition_list_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    if !options.semantic.definition_lists {
        return builder;
    }

    builder = builder.add_handler(vec!["dl"], definition_list_handler);
    builder
}

fn definition_list_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if faithful_with_attrs(h, &e) {
        return h.fallback(e);
    }
    let mut items = Vec::new();
    for child in e.node.children() {
        let Node::Element(element) = child.value() else {
            continue;
        };
        let content = h.walk_children(child).content.trim().to_string();
        if content.is_empty() {
            continue;
        }
        match element.name.local.as_ref() {
            "dt" => items.push(content),
            "dd" => items.push(format!(": {content}")),
            _ => {}
        }
    }
    if items.is_empty() {
        Some("".into())
    } else {
        Some(format!("\n\n{}\n\n", items.join("\n")).into())
    }
}

// ---------------------------------------------------------------------------
// Math
// ---------------------------------------------------------------------------

pub(super) fn add_math_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    if !options.semantic.math.enabled {
        return builder;
    }

    let output = options.semantic.math.output;

    builder = builder.add_handler(vec!["script"], move |h: &dyn Handlers, e: Element<'_>| {
        let ty = e
            .attrs
            .iter()
            .find(|(name, _)| name.local.as_ref() == "type")
            .map(|(_, value)| value.to_string())
            .unwrap_or_default();
        if !ty.starts_with("math/") && !ty.starts_with("text/asciimath") {
            return h.fallback(e);
        }
        let block = ty.contains("mode=display");
        let text = script_text(e.node);
        Some(math_output(&text, block, output).into())
    });

    builder = builder.add_handler(vec!["math"], move |h: &dyn Handlers, e: Element<'_>| {
        if faithful_with_attrs(h, &e) {
            return h.fallback(e);
        }
        let block = e.attrs.iter().any(|(name, value)| {
            name.local.as_ref() == "display" && value.eq_ignore_ascii_case("block")
        });
        let text = mathml_text(e.node);
        Some(math_output(&text, block, output).into())
    });

    builder
}

fn math_output(text: &str, block: bool, mode: MathOutput) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    match mode {
        MathOutput::PreserveHtml => format!("\n\n{text}\n\n"),
        MathOutput::InlineDollar => {
            if block {
                format!("\n\n$${text}$$\n\n")
            } else {
                format!("${text}$")
            }
        }
        MathOutput::BlockDollar => {
            if block {
                format!("\n\n$${text}$$\n\n")
            } else {
                format!("${text}$")
            }
        }
        MathOutput::Fenced => {
            if block {
                format!("\n\n```math\n{text}\n```\n\n")
            } else {
                format!("`{text}`")
            }
        }
        MathOutput::Plain => text.to_string(),
    }
}

fn script_text(node: NodeRef<'_, Node>) -> String {
    node.children()
        .filter_map(|c| c.value().as_text().map(|t| t.text.to_string()))
        .collect()
}

fn mathml_text(node: NodeRef<'_, Node>) -> String {
    fn collect(node: NodeRef<'_, Node>, out: &mut String) {
        if let Node::Text(text) = node.value() {
            out.push_str(&text.text);
        }
        for child in node.children() {
            collect(child, out);
        }
    }
    let mut out = String::new();
    collect(node, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Advanced table handling (data-htmlmd-table markers)
// ---------------------------------------------------------------------------

pub(super) fn add_table_handlers(
    mut builder: NativeConverterBuilder,
    _options: &ConversionOptions,
) -> NativeConverterBuilder {
    builder = builder.add_handler(vec!["table"], table_marker_handler);
    builder
}

fn table_marker_handler(handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
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

pub(super) fn add_mermaid_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    match options.semantic.mermaid {
        MermaidPolicy::Drop => {
            builder = builder
                .add_handler(vec!["pre"], mermaid_drop_handler)
                .add_handler(vec!["div"], mermaid_drop_handler);
        }
        MermaidPolicy::Fenced => {
            builder = builder
                .add_handler(vec!["pre"], mermaid_fenced_handler)
                .add_handler(vec!["div"], mermaid_fenced_handler);
        }
        MermaidPolicy::PreserveHtml => {}
    }
    builder
}

fn mermaid_drop_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if element_has_class(&e, "mermaid") {
        Some("".into())
    } else {
        h.fallback(e)
    }
}

fn mermaid_fenced_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    if !element_has_class(&e, "mermaid") {
        return h.fallback(e);
    }
    let text = h.walk_children(e.node).content.trim().to_string();
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

// ---------------------------------------------------------------------------
// Alerts (GitHub-style markdown alerts)
// ---------------------------------------------------------------------------

pub(super) fn add_alert_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    let extended = matches!(
        options.profile,
        OutputProfile::Extended | OutputProfile::Pandoc | OutputProfile::Obsidian
    );
    if !extended {
        return builder;
    }

    builder = builder
        .add_handler(vec!["div"], alert_handler)
        .add_handler(vec!["blockquote"], alert_handler);

    builder
}

fn alert_handler(handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
    let class = element
        .attrs
        .iter()
        .find(|(name, _)| name.local.as_ref() == "class")
        .map(|(_, value)| value.to_string())
        .unwrap_or_default();
    let alert_type = class
        .split_whitespace()
        .find(|c| c.starts_with("markdown-alert-"))
        .and_then(|c| c.strip_prefix("markdown-alert-"))
        .map(|c| c.to_ascii_uppercase());
    let Some(alert_type) = alert_type else {
        return handlers.fallback(element);
    };

    let body: String = element
        .node
        .children()
        .filter(|c| !is_alert_title(*c))
        .filter_map(|c| handlers.handle(c).map(|r| r.content))
        .collect::<String>()
        .trim()
        .to_string();

    if body.is_empty() {
        return Some(format!("\n\n> [!{alert_type}]\n\n").into());
    }

    let quoted = body
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("\n\n> [!{alert_type}]\n{quoted}\n\n").into())
}

fn is_alert_title(node: NodeRef<'_, Node>) -> bool {
    let Node::Element(element) = node.value() else {
        return false;
    };
    if element.name.local.as_ref() != "p" {
        return false;
    }
    element.attrs.iter().any(|(name, value)| {
        name.local.as_ref() == "class" && value.contains("markdown-alert-title")
    })
}

// ---------------------------------------------------------------------------
// Obsidian wikilinks
// ---------------------------------------------------------------------------

pub(super) fn add_wikilink_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    if options.profile != OutputProfile::Obsidian {
        return builder;
    }
    builder = builder.add_handler(vec!["a"], wikilink_handler);
    builder
}

fn wikilink_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    let is_wikilink = e.attrs.iter().any(|(name, value)| {
        (name.local.as_ref() == "class" && value.split_whitespace().any(|c| c == "wikilink"))
            || (name.local.as_ref() == "rel" && value.as_ref() == "wikilink")
    });
    if !is_wikilink {
        return h.fallback(e);
    }
    let target = e
        .attrs
        .iter()
        .find(|(name, _)| name.local.as_ref() == "href")
        .map(|(_, value)| value.to_string())
        .unwrap_or_default();
    let display = h.walk_children(e.node).content.trim().to_string();
    if display.is_empty() || display == target {
        Some(format!("[[{target}]]").into())
    } else {
        Some(format!("[[{target}|{display}]]").into())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn walk_children(handlers: &dyn Handlers, element: &Element<'_>) -> String {
    handlers.walk_children(element.node).content
}

fn inline_wrap(
    handlers: &dyn Handlers,
    element: Element<'_>,
    prefix: &str,
    suffix: &str,
) -> Option<HandlerResult> {
    if faithful_with_attrs(handlers, &element) {
        return handlers.fallback(element);
    }
    let content = walk_children(handlers, &element);
    if content.trim().is_empty() {
        return Some("".into());
    }
    Some(format!("{prefix}{content}{suffix}").into())
}

fn faithful_with_attrs(handlers: &dyn Handlers, element: &Element<'_>) -> bool {
    handlers.options().translation_mode == TranslationMode::Faithful && !element.attrs.is_empty()
}

// ---------------------------------------------------------------------------
// Reference links / images
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ReferenceState {
    pending: Vec<String>,
    counter: usize,
}

fn next_reference_id(state: &Rc<RefCell<ReferenceState>>) -> usize {
    let mut st = state.borrow_mut();
    st.counter += 1;
    st.counter
}

/// Emit and clear the pending reference definitions (shared by the link and
/// image handlers' `append`).
fn flush_pending_definitions(
    state: &Rc<RefCell<ReferenceState>>,
    placement: ReferencePlacement,
) -> Option<String> {
    if placement == ReferencePlacement::Adjacent {
        return None;
    }
    let defs = std::mem::take(&mut state.borrow_mut().pending);
    if defs.is_empty() {
        None
    } else {
        Some(format!("\n\n{}\n\n", defs.join("\n")))
    }
}

pub(super) fn add_reference_handlers(
    mut builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    let link_ref = matches!(
        options.render.link_style,
        HtmlMdLinkStyle::Reference
            | HtmlMdLinkStyle::CollapsedReference
            | HtmlMdLinkStyle::ShortcutReference
    );
    let image_ref = options.cleanup.image_mode == ImageMode::Reference;
    let placement = options.render.reference_placement;

    if !link_ref && !image_ref {
        return builder;
    }

    // For End placement, links are already handled by the built-in anchor
    // handler. Images still need a custom handler because the core has no
    // reference image support.
    let state = Rc::new(RefCell::new(ReferenceState::default()));

    if link_ref && placement != ReferencePlacement::End {
        builder = builder.add_handler(
            vec!["a"],
            ReferenceLinkHandler {
                state: Rc::clone(&state),
                placement,
            },
        );
    }

    if image_ref {
        builder = builder.add_handler(
            vec!["img"],
            ReferenceImageHandler {
                state: Rc::clone(&state),
                placement,
            },
        );
    }

    if placement == ReferencePlacement::SectionEnd {
        builder = builder.add_handler(
            vec!["h1", "h2", "h3", "h4", "h5", "h6"],
            HeadingFlushHandler { state },
        );
    }

    builder
}

struct ReferenceLinkHandler {
    state: Rc<RefCell<ReferenceState>>,
    placement: ReferencePlacement,
}

impl ElementHandler for ReferenceLinkHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        let mut href: Option<String> = None;
        let mut title: Option<String> = None;
        for (name, value) in element.attrs {
            match name.local.as_ref() {
                "href" => href = Some(value.to_string()),
                "title" => title = Some(value.to_string()),
                _ => {}
            }
        }
        let Some(href) = href else {
            return Some(handlers.walk_children(element.node));
        };

        let text = handlers
            .walk_children(element.node)
            .content
            .trim()
            .replace(']', "\\]");
        let id = format!("ref{}", next_reference_id(&self.state));
        let definition = match title {
            Some(t) if !t.is_empty() => format!("[{id}]: {href} \"{t}\""),
            _ => format!("[{id}]: {href}"),
        };

        let inline = format!("[{text}][{id}]");
        match self.placement {
            ReferencePlacement::Adjacent => Some(format!("{inline}\n\n{definition}\n\n").into()),
            ReferencePlacement::SectionEnd | ReferencePlacement::End => {
                self.state.borrow_mut().pending.push(definition);
                Some(inline.into())
            }
        }
    }

    fn append(&self) -> Option<String> {
        flush_pending_definitions(&self.state, self.placement)
    }
}

struct ReferenceImageHandler {
    state: Rc<RefCell<ReferenceState>>,
    placement: ReferencePlacement,
}

impl ElementHandler for ReferenceImageHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        let mut src: Option<String> = None;
        let mut alt: Option<String> = None;
        let mut title: Option<String> = None;
        for (name, value) in element.attrs {
            match name.local.as_ref() {
                "src" => src = Some(value.to_string()),
                "alt" => alt = Some(value.to_string()),
                "title" => title = Some(value.to_string()),
                _ => {}
            }
        }
        let Some(src) = src else {
            return handlers.fallback(element);
        };

        let alt = alt.unwrap_or_default().replace(']', "\\]");
        let id = format!("img{}", next_reference_id(&self.state));
        let definition = match title {
            Some(t) if !t.is_empty() => format!("[{id}]: {src} \"{t}\""),
            _ => format!("[{id}]: {src}"),
        };

        let inline = format!("![{alt}][{id}]");
        match self.placement {
            ReferencePlacement::Adjacent => Some(format!("{inline}\n\n{definition}\n\n").into()),
            ReferencePlacement::SectionEnd | ReferencePlacement::End => {
                self.state.borrow_mut().pending.push(definition);
                Some(inline.into())
            }
        }
    }

    fn append(&self) -> Option<String> {
        flush_pending_definitions(&self.state, self.placement)
    }
}

struct HeadingFlushHandler {
    state: Rc<RefCell<ReferenceState>>,
}

impl ElementHandler for HeadingFlushHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        if handlers.options().translation_mode == TranslationMode::Faithful
            && !element.attrs.is_empty()
        {
            return handlers.fallback(element);
        }
        let defs = std::mem::take(&mut self.state.borrow_mut().pending);
        let heading = handlers.fallback(element)?.content;
        if defs.is_empty() {
            Some(heading.into())
        } else {
            Some(format!("\n\n{}\n\n{}", defs.join("\n"), heading).into())
        }
    }
}

// ---------------------------------------------------------------------------
// Custom rules
// ---------------------------------------------------------------------------

pub(super) fn add_custom_rule_handlers(
    builder: NativeConverterBuilder,
    options: &ConversionOptions,
) -> NativeConverterBuilder {
    if options.extension.custom_rules.is_empty() {
        return builder;
    }

    // Selector matching and priority resolution happen in the DOM pass
    // (`cleanup::apply_custom_rule_markers`), which renames each claimed
    // element to `htmlmdrule` and records the winning rule's index in
    // `data-htmlmd-rule`. One handler renders them all.
    builder.add_handler(
        vec!["htmlmdrule"],
        CustomRuleHandler {
            rules: options.extension.custom_rules.clone(),
        },
    )
}

struct CustomRuleHandler {
    rules: Vec<CustomRule>,
}

impl ElementHandler for CustomRuleHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        let rule = element
            .attrs
            .iter()
            .find(|(name, _)| name.local.as_ref() == "data-htmlmd-rule")
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .and_then(|index| self.rules.get(index));
        let Some(rule) = rule else {
            return handlers.fallback(element);
        };

        match rule.action {
            CustomRuleAction::MarkdownTemplate => {
                let template = rule.template.as_deref()?;
                let text = handlers
                    .walk_children(element.node)
                    .content
                    .trim()
                    .to_string();
                static ATTR_RE: once_cell::sync::Lazy<regex::Regex> =
                    once_cell::sync::Lazy::new(|| regex::Regex::new(r"\{attr:([\w-]+)\}").unwrap());
                let mut result = template.replace("{text}", &text);
                for caps in ATTR_RE.captures_iter(template) {
                    let placeholder = &caps[0];
                    let attr_name = &caps[1];
                    let value = element
                        .attrs
                        .iter()
                        .find(|(name, _)| name.local.as_ref() == attr_name)
                        .map(|(_, value)| value.to_string())
                        .unwrap_or_default();
                    result = result.replace(placeholder, &value);
                }
                Some(result.into())
            }
            CustomRuleAction::FencedBlock => {
                let text = handlers
                    .walk_children(element.node)
                    .content
                    .trim()
                    .to_string();
                if text.is_empty() {
                    return Some("".into());
                }
                let lang = rule.template.as_deref().unwrap_or("");
                let fence = make_code_fence(&text, 3);
                Some(format!("\n\n{fence}{lang}\n{text}\n{fence}\n\n").into())
            }
            CustomRuleAction::Link => {
                let href = element
                    .attrs
                    .iter()
                    .find(|(name, _)| name.local.as_ref() == "href")
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_default();
                let text = handlers
                    .walk_children(element.node)
                    .content
                    .trim()
                    .to_string();
                let dest = if href.is_empty() { text.clone() } else { href };
                let label = text.replace(']', "\\]");
                let dest = dest.replace('(', "\\(").replace(')', "\\)");
                Some(format!("[{label}]({dest})").into())
            }
            CustomRuleAction::Image => {
                let src = element
                    .attrs
                    .iter()
                    .find(|(name, _)| name.local.as_ref() == "src")
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_default();
                let alt = element
                    .attrs
                    .iter()
                    .find(|(name, _)| name.local.as_ref() == "alt")
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_else(|| {
                        handlers
                            .walk_children(element.node)
                            .content
                            .trim()
                            .to_string()
                    });
                if src.is_empty() {
                    return Some("".into());
                }
                let alt = alt.replace(']', "\\]");
                let src = src.replace('(', "\\(").replace(')', "\\)");
                Some(format!("![{alt}]({src})").into())
            }
            _ => handlers.fallback(element),
        }
    }
}

fn make_code_fence(content: &str, min_len: usize) -> String {
    let mut len = min_len;
    while content.contains(&"`".repeat(len)) {
        len += 1;
    }
    "`".repeat(len)
}

// SPDX-License-Identifier: MIT OR Apache-2.0

//! Custom `htmd` element handlers that implement Extended-profile features.
//!
//! This module hides `htmd`-specific APIs from the rest of `htmlmd-core`.  All
//! handlers are registered on a per-conversion basis so that `ConversionOptions`
//! can control which Markdown extensions are emitted.

use htmd::element_handler::{HandlerResult, Handlers};
use htmd::options::{
    BrStyle, BulletListMarker, CodeBlockFence, HeadingStyle as HtmdHeadingStyle, HrStyle,
    LinkReferenceStyle, LinkStyle, Options as HtmdOptions, TranslationMode,
};
use htmd::{Element, HtmlToMarkdown, HtmlToMarkdownBuilder};
use html5ever::serialize::{SerializeOpts, TraversalScope, serialize};
use markup5ever_rcdom::{Node, NodeData};
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::options::{
    BulletMarker, CodeFence, ConversionOptions, CustomRule, CustomRuleAction, HardBreakStyle,
    HeadingStyle, HrStyle as HtmlMdHrStyle, ImageMode, LinkStyle as HtmlMdLinkStyle, MathOutput,
    MermaidPolicy, OutputProfile, RawHtmlPolicy, ReferencePlacement,
};
use std::sync::{Arc, Mutex};

/// Build a fully configured `HtmlToMarkdown` converter from `ConversionOptions`.
pub fn build_converter(options: &ConversionOptions) -> HtmlToMarkdown {
    let htmd_options = build_htmd_options(options);
    let scripting_enabled = options.cleanup.remove_tags.iter().all(|t| t != "noscript");

    let mut builder = HtmlToMarkdown::builder()
        .options(htmd_options)
        .scripting_enabled(scripting_enabled);

    let skip_tags: Vec<&str> = options.cleanup.remove_tags.iter().map(|s| s.as_str()).collect();
    if !skip_tags.is_empty() {
        builder = builder.skip_tags(skip_tags);
    }

    builder = add_semantic_handlers(builder, options);
    builder = add_footnote_handlers(builder, options);
    builder = add_definition_list_handlers(builder, options);
    builder = add_table_handlers(builder, options);
    builder = add_math_handlers(builder, options);
    builder = add_mermaid_handlers(builder, options);
    builder = add_alert_handlers(builder, options);
    builder = add_wikilink_handlers(builder, options);
    builder = add_reference_handlers(builder, options);
    builder = add_custom_rule_handlers(builder, options);

    builder.build()
}

#[allow(clippy::field_reassign_with_default)]
fn build_htmd_options(options: &ConversionOptions) -> HtmdOptions {
    let mut o = HtmdOptions::default();

    o.heading_style = match options.render.heading_style {
        HeadingStyle::Atx | HeadingStyle::Keep => HtmdHeadingStyle::Atx,
        HeadingStyle::Setex => HtmdHeadingStyle::Setex,
    };

    o.hr_style = match options.render.hr_style {
        HtmlMdHrStyle::Dashes => HrStyle::Dashes,
        HtmlMdHrStyle::Asterisks => HrStyle::Asterisks,
        HtmlMdHrStyle::Underscores => HrStyle::Underscores,
    };

    o.br_style = match options.render.hard_break_style {
        HardBreakStyle::TwoSpaces => BrStyle::TwoSpaces,
        HardBreakStyle::Backslash => BrStyle::Backslash,
    };

    o.bullet_list_marker = match options.render.bullet_marker {
        BulletMarker::Asterisk => BulletListMarker::Asterisk,
        BulletMarker::Hyphen | BulletMarker::Plus => BulletListMarker::Dash,
    };

    o.code_block_fence = match options.render.code_fence {
        CodeFence::Backticks => CodeBlockFence::Backticks,
        CodeFence::Tildes => CodeBlockFence::Tildes,
    };

    o.link_style = match options.render.link_style {
        HtmlMdLinkStyle::Inline => LinkStyle::Inlined,
        HtmlMdLinkStyle::Reference
        | HtmlMdLinkStyle::CollapsedReference
        | HtmlMdLinkStyle::ShortcutReference => LinkStyle::Referenced,
    };

    o.link_reference_style = match options.render.reference_placement {
        ReferencePlacement::End | ReferencePlacement::SectionEnd => LinkReferenceStyle::Full,
        ReferencePlacement::Adjacent => LinkReferenceStyle::Full,
    };

    match options.render.link_style {
        HtmlMdLinkStyle::CollapsedReference => {
            o.link_reference_style = LinkReferenceStyle::Collapsed;
        }
        HtmlMdLinkStyle::ShortcutReference => {
            o.link_reference_style = LinkReferenceStyle::Shortcut;
        }
        _ => {}
    }

    o.translation_mode = match options.render.raw_html_policy {
        RawHtmlPolicy::Faithful => TranslationMode::Faithful,
        _ => TranslationMode::Pure,
    };

    o
}

// ---------------------------------------------------------------------------
// Semantic handlers
// ---------------------------------------------------------------------------

fn add_semantic_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
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
// Footnotes
// ---------------------------------------------------------------------------

fn add_footnote_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
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
        .find(|a| a.name.local.as_ref() == "id")
        .map(|a| a.value.to_string());
    let Some(id) = id else {
        return h.fallback(e);
    };
    let Some(label) = id.strip_prefix("fn").map(normalize_footnote_label) else {
        return h.fallback(e);
    };
    let content = footnote_definition_content(h, e.node);
    Some(format!("\n\n[^{label}]: {}\n\n", content.trim()).into())
}

fn footnote_reference_label(node: &Rc<Node>) -> Option<String> {
    let children = node.children.borrow();
    if children.len() != 1 {
        return None;
    }
    let child = children.first()?;
    let NodeData::Element { name, attrs, .. } = &child.data else {
        return None;
    };
    if name.local.as_ref() != "a" {
        return None;
    }
    let href = attrs
        .borrow()
        .iter()
        .find(|a| a.name.local.as_ref() == "href")
        .map(|a| a.value.to_string())?;
    href.strip_prefix("#fn").map(normalize_footnote_label)
}

fn footnote_definition_content(handlers: &dyn Handlers, node: &Rc<Node>) -> String {
    node.children
        .borrow()
        .iter()
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

fn is_backlink(node: &Rc<Node>) -> bool {
    let NodeData::Element { name, attrs, .. } = &node.data else {
        return false;
    };
    if name.local.as_ref() != "a" {
        return false;
    }
    attrs
        .borrow()
        .iter()
        .any(|a| a.name.local.as_ref() == "href" && a.value.starts_with("#fnref"))
}

// ---------------------------------------------------------------------------
// Definition lists
// ---------------------------------------------------------------------------

fn add_definition_list_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
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
    for child in e.node.children.borrow().iter() {
        let NodeData::Element { name, .. } = &child.data else {
            continue;
        };
        let content = h.walk_children(child).content.trim().to_string();
        if content.is_empty() {
            continue;
        }
        match name.local.as_ref() {
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

fn add_math_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
    if !options.semantic.math.enabled {
        return builder;
    }

    let output = options.semantic.math.output;

    builder = builder.add_handler(vec!["script"], move |h: &dyn Handlers, e: Element<'_>| {
        let ty = e
            .attrs
            .iter()
            .find(|a| a.name.local.as_ref() == "type")
            .map(|a| a.value.to_string())
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
        let block = e.attrs.iter().any(|a| {
            a.name.local.as_ref() == "display" && a.value.eq_ignore_ascii_case("block")
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

fn script_text(node: &Rc<Node>) -> String {
    node.children
        .borrow()
        .iter()
        .filter_map(|c| {
            if let NodeData::Text { contents } = &c.data {
                Some(contents.borrow().to_string())
            } else {
                None
            }
        })
        .collect()
}

fn mathml_text(node: &Rc<Node>) -> String {
    fn collect(node: &Rc<Node>, out: &mut String) {
        if let NodeData::Text { contents } = &node.data {
            out.push_str(&contents.borrow());
        }
        for child in node.children.borrow().iter() {
            collect(child, out);
        }
    }
    let mut out = String::new();
    collect(node, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Advanced table handling
// ---------------------------------------------------------------------------

fn add_table_handlers(
    mut builder: HtmlToMarkdownBuilder,
    _options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
    builder = builder.add_handler(vec!["table"], table_handler);
    builder
}

fn table_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    let marker = e
        .attrs
        .iter()
        .find(|a| a.name.local.as_ref() == "data-htmlmd-table")
        .map(|a| a.value.to_string());

    match marker.as_deref() {
        Some("html") => {
            let html = serialize_node_to_html(e.node)
                .replace(" data-htmlmd-table=\"html\"", "")
                .replace(" data-htmlmd-table=\"csv\"", "");
            if html.is_empty() {
                return Some("".into());
            }
            let fence = make_code_fence(&html, 3);
            Some(format!("\n\n{fence}html\n{html}\n{fence}\n\n").into())
        }
        Some("csv") => {
            let csv = table_to_csv(e.node);
            if csv.is_empty() {
                return Some("".into());
            }
            let fence = make_code_fence(&csv, 3);
            Some(format!("\n\n{fence}csv\n{csv}\n{fence}\n\n").into())
        }
        _ => h.fallback(e),
    }
}

fn serialize_node_to_html(node: &Rc<Node>) -> String {
    let mut bytes = Vec::new();
    let opts = SerializeOpts {
        traversal_scope: TraversalScope::IncludeNode,
        ..Default::default()
    };
    if serialize(
        &mut bytes,
        &markup5ever_rcdom::SerializableHandle::from(node.clone()),
        opts,
    )
    .is_ok()
    {
        String::from_utf8(bytes).unwrap_or_default()
    } else {
        String::new()
    }
}

fn table_to_csv(node: &Rc<Node>) -> String {
    fn collect_text(node: &Rc<Node>, out: &mut String) {
        if let NodeData::Text { contents } = &node.data {
            out.push_str(&contents.borrow());
        }
        for child in node.children.borrow().iter() {
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

    fn row_to_csv(node: &Rc<Node>) -> Option<String> {
        let NodeData::Element { name, .. } = &node.data else {
            return None;
        };
        if name.local.as_ref() != "tr" {
            return None;
        }
        let cells: Vec<String> = node
            .children
            .borrow()
            .iter()
            .filter_map(|c| {
                let NodeData::Element { name, .. } = &c.data else {
                    return None;
                };
                if name.local.as_ref() == "td" || name.local.as_ref() == "th" {
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

    fn collect_rows(node: &Rc<Node>, rows: &mut Vec<String>) {
        let NodeData::Element { name, .. } = &node.data else {
            for child in node.children.borrow().iter() {
                collect_rows(child, rows);
            }
            return;
        };
        if name.local.as_ref() == "tr" {
            if let Some(line) = row_to_csv(node) {
                rows.push(line);
            }
            return;
        }
        for child in node.children.borrow().iter() {
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

fn add_mermaid_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
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
        .find(|a| a.name.local.as_ref() == "class")
        .map(|a| a.value.split_whitespace().any(|c| c == class))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Alerts (GitHub-style markdown alerts)
// ---------------------------------------------------------------------------

fn add_alert_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
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
        .find(|a| a.name.local.as_ref() == "class")
        .map(|a| a.value.to_string())
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
        .children
        .borrow()
        .iter()
        .filter(|c| !is_alert_title(c))
        .filter_map(|c| handlers.handle(c).map(|r| r.content))
        .collect::<String>()
        .trim()
        .to_string();

    if body.is_empty() {
        return Some(format!("\n\n> [!{alert_type}]\n\n").into());
    }

    let quoted = body
        .lines()
        .map(|line| if line.is_empty() { ">".to_string() } else { format!("> {line}") })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("\n\n> [!{alert_type}]\n{quoted}\n\n").into())
}

fn is_alert_title(node: &Rc<Node>) -> bool {
    let NodeData::Element { name, attrs, .. } = &node.data else {
        return false;
    };
    if name.local.as_ref() != "p" {
        return false;
    }
    attrs
        .borrow()
        .iter()
        .any(|a| a.name.local.as_ref() == "class" && a.value.contains("markdown-alert-title"))
}

// ---------------------------------------------------------------------------
// Obsidian wikilinks
// ---------------------------------------------------------------------------

fn add_wikilink_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
    if options.profile != OutputProfile::Obsidian {
        return builder;
    }
    builder = builder.add_handler(vec!["a"], wikilink_handler);
    builder
}

fn wikilink_handler(h: &dyn Handlers, e: Element<'_>) -> Option<HandlerResult> {
    let is_wikilink = e.attrs.iter().any(|a| {
        (a.name.local.as_ref() == "class"
            && a.value.split_whitespace().any(|c| c == "wikilink"))
            || (a.name.local.as_ref() == "rel" && a.value.as_ref() == "wikilink")
    });
    if !is_wikilink {
        return h.fallback(e);
    }
    let target = e
        .attrs
        .iter()
        .find(|a| a.name.local.as_ref() == "href")
        .map(|a| a.value.to_string())
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

fn next_reference_id(state: &Mutex<ReferenceState>) -> usize {
    let mut st = state.lock().unwrap();
    st.counter += 1;
    st.counter
}

fn add_reference_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
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

    // For End placement, links are already handled by htmd's anchor handler.
    // Images still need a custom handler because htmd has no reference image support.
    let state = Arc::new(Mutex::new(ReferenceState::default()));

    if link_ref && placement != ReferencePlacement::End {
        builder = builder.add_handler(
            vec!["a"],
            ReferenceLinkHandler {
                state: Arc::clone(&state),
                placement,
            },
        );
    }

    if image_ref {
        builder = builder.add_handler(
            vec!["img"],
            ReferenceImageHandler {
                state: Arc::clone(&state),
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

#[derive(Clone)]
struct ReferenceLinkHandler {
    state: Arc<Mutex<ReferenceState>>,
    placement: ReferencePlacement,
}

impl htmd::element_handler::ElementHandler for ReferenceLinkHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        let mut href: Option<String> = None;
        let mut title: Option<String> = None;
        for attr in element.attrs.iter() {
            match attr.name.local.as_ref() {
                "href" => href = Some(attr.value.to_string()),
                "title" => title = Some(attr.value.to_string()),
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
            ReferencePlacement::Adjacent => {
                Some(format!("{inline}\n\n{definition}\n\n").into())
            }
            ReferencePlacement::SectionEnd | ReferencePlacement::End => {
                self.state.lock().unwrap().pending.push(definition);
                Some(inline.into())
            }
        }
    }

    fn append(&self) -> Option<String> {
        if self.placement == ReferencePlacement::Adjacent {
            return None;
        }
        let defs = {
            let mut st = self.state.lock().unwrap();
            std::mem::take(&mut st.pending)
        };
        if defs.is_empty() {
            None
        } else {
            Some(format!("\n\n{}\n\n", defs.join("\n")))
        }
    }
}

#[derive(Clone)]
struct ReferenceImageHandler {
    state: Arc<Mutex<ReferenceState>>,
    placement: ReferencePlacement,
}

impl htmd::element_handler::ElementHandler for ReferenceImageHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        let mut src: Option<String> = None;
        let mut alt: Option<String> = None;
        let mut title: Option<String> = None;
        for attr in element.attrs.iter() {
            match attr.name.local.as_ref() {
                "src" => src = Some(attr.value.to_string()),
                "alt" => alt = Some(attr.value.to_string()),
                "title" => title = Some(attr.value.to_string()),
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
            ReferencePlacement::Adjacent => {
                Some(format!("{inline}\n\n{definition}\n\n").into())
            }
            ReferencePlacement::SectionEnd | ReferencePlacement::End => {
                self.state.lock().unwrap().pending.push(definition);
                Some(inline.into())
            }
        }
    }

    fn append(&self) -> Option<String> {
        if self.placement == ReferencePlacement::Adjacent {
            return None;
        }
        let defs = {
            let mut st = self.state.lock().unwrap();
            std::mem::take(&mut st.pending)
        };
        if defs.is_empty() {
            None
        } else {
            Some(format!("\n\n{}\n\n", defs.join("\n")))
        }
    }
}

#[derive(Clone)]
struct HeadingFlushHandler {
    state: Arc<Mutex<ReferenceState>>,
}

impl htmd::element_handler::ElementHandler for HeadingFlushHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        if handlers.options().translation_mode == TranslationMode::Faithful && !element.attrs.is_empty()
        {
            return handlers.fallback(element);
        }
        let defs = {
            let mut st = self.state.lock().unwrap();
            std::mem::take(&mut st.pending)
        };
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

fn add_custom_rule_handlers(
    mut builder: HtmlToMarkdownBuilder,
    options: &ConversionOptions,
) -> HtmlToMarkdownBuilder {
    if options.extension.custom_rules.is_empty() {
        return builder;
    }

    let mut indexed: Vec<_> = options.extension.custom_rules.iter().enumerate().collect();
    // Register lower priority first so higher priority handlers are consulted last
    // (and therefore win when htmd searches handlers in reverse order).
    indexed.sort_by(|a, b| a.1.priority.cmp(&b.1.priority));

    for (_index, rule) in indexed {
        for selector in &rule.selectors {
            if let Some(tag) = simple_selector_tag(selector) {
                builder = builder.add_handler(
                    vec![tag.as_str()],
                    CustomRuleHandler {
                        selector: selector.clone(),
                        rule: rule.clone(),
                    },
                );
            }
        }
    }

    builder
}

#[derive(Debug, Clone)]
struct CustomRuleHandler {
    selector: String,
    rule: CustomRule,
}

impl htmd::element_handler::ElementHandler for CustomRuleHandler {
    fn handle(&self, handlers: &dyn Handlers, element: Element<'_>) -> Option<HandlerResult> {
        if !element_matches_simple_selector(&element, &self.selector) {
            return handlers.fallback(element);
        }

        match self.rule.action {
            CustomRuleAction::MarkdownTemplate => {
                let template = self.rule.template.as_deref()?;
                let text = handlers.walk_children(element.node).content.trim().to_string();
                let mut result = template.replace("{text}", &text);
                if let Ok(attr_re) = regex::Regex::new(r"\{attr:(\w+)\}") {
                    for caps in attr_re.captures_iter(template) {
                        let placeholder = &caps[0];
                        let attr_name = &caps[1];
                        let value = element
                            .attrs
                            .iter()
                            .find(|a| a.name.local.as_ref() == attr_name)
                            .map(|a| a.value.to_string())
                            .unwrap_or_default();
                        result = result.replace(placeholder, &value);
                    }
                }
                Some(result.into())
            }
            CustomRuleAction::FencedBlock => {
                let text = handlers.walk_children(element.node).content.trim().to_string();
                if text.is_empty() {
                    return Some("".into());
                }
                let lang = self.rule.template.as_deref().unwrap_or("");
                let fence = make_code_fence(&text, 3);
                Some(format!("\n\n{fence}{lang}\n{text}\n{fence}\n\n").into())
            }
            CustomRuleAction::Link => {
                let href = element
                    .attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "href")
                    .map(|a| a.value.to_string())
                    .unwrap_or_default();
                let text = handlers.walk_children(element.node).content.trim().to_string();
                let dest = if href.is_empty() { text.clone() } else { href };
                let label = text.replace(']', "\\]");
                let dest = dest.replace('(', "\\(").replace(')', "\\)");
                Some(format!("[{label}]({dest})").into())
            }
            CustomRuleAction::Image => {
                let src = element
                    .attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "src")
                    .map(|a| a.value.to_string())
                    .unwrap_or_default();
                let alt = element
                    .attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "alt")
                    .map(|a| a.value.to_string())
                    .unwrap_or_else(|| {
                        handlers.walk_children(element.node).content.trim().to_string()
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

fn simple_selector_tag(selector: &str) -> Option<String> {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return None;
    }
    let first = trimmed.chars().next().unwrap();
    if matches!(first, '.' | '#' | '[' | '*') {
        return None;
    }
    let end = trimmed
        .find(['.', '#', '[', ':', ' ', '>', '+', '~'].as_slice())
        .unwrap_or(trimmed.len());
    let tag = &trimmed[..end];
    if tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Some(tag.to_ascii_lowercase())
    } else {
        None
    }
}

fn element_matches_simple_selector(element: &Element<'_>, selector: &str) -> bool {
    let trimmed = selector.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut rest = trimmed;
    // Optional tag.
    let first = rest.chars().next().unwrap();
    if first.is_ascii_alphabetic() {
        let end = rest
            .find(['.', '#', '[', ':', ' ', '>', '+', '~'].as_slice())
            .unwrap_or(rest.len());
        let tag = &rest[..end];
        if !element.tag.eq_ignore_ascii_case(tag) {
            return false;
        }
        rest = &rest[end..];
    }

    let mut required_classes = Vec::new();
    let mut required_id: Option<&str> = None;

    while !rest.is_empty() {
        match rest.chars().next().unwrap() {
            '.' => {
                let end = rest[1..]
                    .find(['.', '#', '[', ':', ' ', '>', '+', '~'].as_slice())
                    .map(|i| i + 1)
                    .unwrap_or(rest.len());
                required_classes.push(&rest[1..end]);
                rest = &rest[end..];
            }
            '#' => {
                let end = rest[1..]
                    .find(['.', '#', '[', ':', ' ', '>', '+', '~'].as_slice())
                    .map(|i| i + 1)
                    .unwrap_or(rest.len());
                required_id = Some(&rest[1..end]);
                rest = &rest[end..];
            }
            _ => break,
        }
    }

    let class_attr = element
        .attrs
        .iter()
        .find(|a| a.name.local.as_ref() == "class")
        .map(|a| a.value.to_string())
        .unwrap_or_default();
    let classes: Vec<_> = class_attr.split_whitespace().collect();
    for cls in required_classes {
        if !classes.contains(&cls) {
            return false;
        }
    }

    if let Some(id) = required_id {
        let element_id = element
            .attrs
            .iter()
            .find(|a| a.name.local.as_ref() == "id")
            .map(|a| a.value.as_ref());
        if element_id != Some(id) {
            return false;
        }
    }

    true
}

fn make_code_fence(content: &str, min_len: usize) -> String {
    let mut len = min_len;
    while content.contains(&"`".repeat(len)) {
        len += 1;
    }
    "`".repeat(len)
}

/// Convert HTML to Markdown using the default backend and return a `Result`.
///
/// This is a small adapter used by `HtmdBackend` so that the backend only deals
/// with the public `ConverterBackend` trait.
pub fn convert_with_htmd(html: &str, options: &ConversionOptions) -> Result<String> {
    build_converter(options)
        .convert(html)
        .map_err(|e| Error::Parse(e.to_string()))
}

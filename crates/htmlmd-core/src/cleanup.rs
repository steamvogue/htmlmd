// SPDX-License-Identifier: MIT OR Apache-2.0

use ego_tree::{NodeId, NodeRef};
use markup5ever::{LocalName, QualName};
use scraper::{Html, Node, Selector};

use crate::diagnostic::{Diagnostic, DiagnosticsCollector};
use crate::error::{Error, Result};
use crate::options::{
    ConversionOptions, CustomElementPolicy, CustomRuleAction, DetailsHandling,
    DifficultTableStrategy, FormHandling, HiddenContentPolicy, ImageMode, MediaPolicy,
    TableHandling, TitleHandling,
};
use crate::rewrite::{CompiledRewriteRules, choose_largest_srcset, rewrite_url_attr};
use once_cell::sync::Lazy;

// Selectors used inside per-table / per-row loops; parsed once per process.
static SEL_TABLE: Lazy<Selector> = Lazy::new(|| Selector::parse("table").unwrap());
static SEL_NESTED_TABLE: Lazy<Selector> = Lazy::new(|| Selector::parse("table table").unwrap());
static SEL_TR: Lazy<Selector> = Lazy::new(|| Selector::parse("tr").unwrap());
static SEL_CELLS: Lazy<Selector> = Lazy::new(|| Selector::parse("td, th").unwrap());
static SEL_SPAN_CELLS: Lazy<Selector> =
    Lazy::new(|| Selector::parse("[rowspan], [colspan]").unwrap());

/// Apply HTML cleanup, content selection, and URL rewriting to a string of HTML.
///
/// Returns the cleaned HTML string and extracted metadata. This is a thin
/// serialize wrapper over [`clean_html_to_dom`]; backends that implement
/// `ConverterBackend::convert_dom` natively can consume the DOM directly and
/// skip the serialize/re-parse round trip.
pub fn clean_html(
    html: &str,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<(String, ExtractedMetadata)> {
    let (document, metadata) = clean_html_to_dom(html, options, path, diagnostics)?;
    Ok((document.html(), metadata))
}

/// Apply HTML cleanup, content selection, and URL rewriting, returning the
/// cleaned document as a parsed `scraper::Html` tree plus extracted metadata.
pub fn clean_html_to_dom(
    html: &str,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<(Html, ExtractedMetadata)> {
    let mut document = Html::parse_document(html);

    check_limits(html, &document, options, diagnostics)?;

    // Extract metadata from the original document before any destructive cleanup.
    let metadata = extract_metadata(&document, options);

    remove_hidden(&mut document, options);
    apply_remove_tags(&mut document, options);
    apply_footnote_cleanup(&mut document, options);
    apply_per_tag_behavior(&mut document, options);
    apply_heading_offset(&mut document, options);
    apply_remove_selectors(&mut document, options, path, diagnostics)?;
    apply_unwrap_selectors(&mut document, options, path, diagnostics)?;
    apply_keep_only(&mut document, options, path, diagnostics)?;
    apply_custom_rules(&mut document, options, path, diagnostics)?;
    apply_details_handling(&mut document, options);
    apply_form_handling(&mut document, options);
    apply_custom_elements(&mut document, options);
    apply_table_handling(&mut document, options);
    apply_media_policy(&mut document, options);
    apply_code_language_detection(&mut document, options);
    apply_image_handling(&mut document, options);
    apply_custom_rule_markers(&mut document, options, path, diagnostics)?;
    apply_url_rewriting(&mut document, options, diagnostics)?;

    if options.render.title_attribute == TitleHandling::Ignore {
        strip_attribute(&mut document, "title");
    }

    Ok((document, metadata))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub open_graph_title: Option<String>,
    pub open_graph_description: Option<String>,
    pub open_graph_image: Option<String>,
    pub twitter_title: Option<String>,
    pub twitter_description: Option<String>,
}

fn check_limits(
    html: &str,
    document: &Html,
    options: &ConversionOptions,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    let limits = &options.limits;
    if limits.max_input_bytes > 0 && html.len() as u64 > limits.max_input_bytes {
        let msg = format!(
            "input size {} exceeds limit {}",
            html.len(),
            limits.max_input_bytes
        );
        if options.strict {
            return Err(Error::LimitExceeded(msg));
        }
        diagnostics.push(Diagnostic::warning(msg));
    }
    if limits.max_node_count > 0 {
        let count = document.tree.nodes().count() as u64;
        if count > limits.max_node_count {
            let msg = format!(
                "DOM node count {count} exceeds limit {}",
                limits.max_node_count
            );
            if options.strict {
                return Err(Error::LimitExceeded(msg));
            }
            diagnostics.push(Diagnostic::warning(msg));
        }
    }
    if limits.max_dom_depth > 0 {
        let depth = max_dom_depth(document);
        if depth > limits.max_dom_depth {
            let msg = format!("DOM depth {depth} exceeds limit {}", limits.max_dom_depth);
            if options.strict {
                return Err(Error::LimitExceeded(msg));
            }
            diagnostics.push(Diagnostic::warning(msg));
        }
    }
    if limits.max_attribute_len > 0 {
        let attr_len = max_attribute_len(document);
        if attr_len > limits.max_attribute_len {
            let msg = format!(
                "attribute length {attr_len} exceeds limit {}",
                limits.max_attribute_len
            );
            if options.strict {
                return Err(Error::LimitExceeded(msg));
            }
            diagnostics.push(Diagnostic::warning(msg));
        }
    }
    Ok(())
}

fn max_dom_depth(document: &Html) -> u32 {
    fn depth(node: NodeRef<'_, Node>, current: u32) -> u32 {
        let mut max = current;
        for child in node.children() {
            max = max.max(depth(child, current + 1));
        }
        max
    }
    document
        .tree
        .root()
        .children()
        .map(|c| depth(c, 1))
        .max()
        .unwrap_or(0)
}

fn max_attribute_len(document: &Html) -> u64 {
    document
        .tree
        .nodes()
        .filter_map(|n| n.value().as_element())
        .flat_map(|el| el.attrs.iter().map(|(_, v)| v.len() as u64))
        .max()
        .unwrap_or(0)
}

fn apply_remove_tags(document: &mut Html, options: &ConversionOptions) {
    for tag in &options.cleanup.remove_tags {
        let selector_str = if tag.eq_ignore_ascii_case("script") && options.semantic.math.enabled {
            r#"script:not([type^="math/"]):not([type^="text/asciimath"])"#
        } else {
            tag
        };
        let Ok(selector) = Selector::parse(selector_str) else {
            continue;
        };
        detach_selected(document, &selector);
    }
}

fn apply_footnote_cleanup(document: &mut Html, options: &ConversionOptions) {
    if !options.semantic.footnotes {
        return;
    }
    let selectors = [".footnotes ol", ".footnote-list", "ol.footnotes"];
    for s in selectors {
        let Ok(selector) = Selector::parse(s) else {
            continue;
        };
        let ids: Vec<NodeId> = document.select(&selector).map(|e| e.id()).collect();
        for id in ids {
            unwrap_node(document, id);
        }
    }
}

fn apply_remove_selectors(
    document: &mut Html,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    for s in &options.cleanup.remove_selectors {
        match Selector::parse(s) {
            Ok(selector) => detach_selected(document, &selector),
            Err(e) => {
                let mut d = Diagnostic::warning(format!("invalid remove selector {s}: {e}"));
                if let Some(p) = path {
                    d = d.with_path(p);
                }
                diagnostics.push(d);
                if options.strict {
                    return Err(Error::Selector(format!("invalid selector {s}: {e}")));
                }
            }
        }
    }
    Ok(())
}

fn apply_unwrap_selectors(
    document: &mut Html,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    for s in &options.cleanup.unwrap_selectors {
        match Selector::parse(s) {
            Ok(selector) => unwrap_selected(document, &selector),
            Err(e) => {
                let mut d = Diagnostic::warning(format!("invalid unwrap selector {s}: {e}"));
                if let Some(p) = path {
                    d = d.with_path(p);
                }
                diagnostics.push(d);
                if options.strict {
                    return Err(Error::Selector(format!("invalid selector {s}: {e}")));
                }
            }
        }
    }
    Ok(())
}

fn apply_keep_only(
    document: &mut Html,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    let selectors: Vec<&String> = options
        .cleanup
        .extract_selector
        .iter()
        .chain(options.cleanup.main_content_selector.iter())
        .chain(options.cleanup.keep_only_selectors.iter())
        .collect();

    if selectors.is_empty() {
        return Ok(());
    }

    let mut chosen: Option<String> = None;
    for s in selectors {
        match Selector::parse(s) {
            Ok(selector) => {
                if let Some(el) = document.select(&selector).next() {
                    chosen = Some(el.html());
                    break;
                }
            }
            Err(e) => {
                let mut d = Diagnostic::warning(format!("invalid keep selector {s}: {e}"));
                if let Some(p) = path {
                    d = d.with_path(p);
                }
                diagnostics.push(d);
                if options.strict {
                    return Err(Error::Selector(format!("invalid selector {s}: {e}")));
                }
            }
        }
    }

    if let Some(fragment_html) = chosen {
        *document = Html::parse_document(&fragment_html);
    }

    Ok(())
}

fn remove_hidden(document: &mut Html, options: &ConversionOptions) {
    if options.cleanup.hidden_content_policy == HiddenContentPolicy::Show {
        return;
    }

    let selectors = [
        "[hidden]",
        "[aria-hidden=\"true\"]",
        "[style*=\"display:none\"]",
        "[style*=\"display: none\"]",
        "[style*=\"visibility:hidden\"]",
        "[style*=\"visibility: hidden\"]",
    ];
    for s in selectors {
        if let Ok(selector) = Selector::parse(s) {
            detach_selected(document, &selector);
        }
    }
}

fn apply_url_rewriting(
    document: &mut Html,
    options: &ConversionOptions,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    let base = options
        .cleanup
        .base_url
        .as_deref()
        .and_then(|u| url::Url::parse(u).ok());

    // Compile rewrite-rule regexes once per conversion, not per URL.
    let rules = CompiledRewriteRules::from_options(options);

    let url_attrs = ["href", "src", "srcset"];
    for attr in url_attrs {
        let selector_str = format!("[{attr}]");
        let Ok(selector) = Selector::parse(&selector_str) else {
            continue;
        };
        let matches: Vec<_> = document.select(&selector).map(|e| (e.id(), attr)).collect();
        for (id, attr_name) in matches {
            let Some(mut node) = document.tree.get_mut(id) else {
                continue;
            };
            let Node::Element(ref mut el) = *node.value() else {
                continue;
            };
            let Some(original) = el
                .attrs
                .iter()
                .find(|(name, _)| name.local.as_ref() == attr_name)
                .map(|(_, v)| v.to_string())
            else {
                continue;
            };
            let rewritten = rewrite_url_attr(
                &original,
                attr_name,
                base.as_ref(),
                options,
                &rules,
                diagnostics,
            );
            set_attr(el, attr_name, &rewritten);
        }
    }
    Ok(())
}

fn strip_attribute(document: &mut Html, attr_name: &str) {
    let Ok(selector) = Selector::parse(&format!("[{attr_name}]")) else {
        return;
    };
    let ids: Vec<_> = document.select(&selector).map(|e| e.id()).collect();
    for id in ids {
        let Some(mut node) = document.tree.get_mut(id) else {
            continue;
        };
        let Node::Element(ref mut el) = *node.value() else {
            continue;
        };
        remove_attr(el, attr_name);
    }
}

fn set_attr(el: &mut scraper::node::Element, attr_name: &str, value: &str) {
    let name = QualName::new(None, markup5ever::ns!(), LocalName::from(attr_name));
    if let Some(pos) = el.attrs.iter().position(|(n, _)| n == &name) {
        el.attrs[pos].1 = value.into();
    } else {
        el.attrs.push((name, value.into()));
    }
}

fn remove_attr(el: &mut scraper::node::Element, attr_name: &str) {
    let name = QualName::new(None, markup5ever::ns!(), LocalName::from(attr_name));
    el.attrs.retain(|(n, _)| n != &name);
}

fn detach_selected(document: &mut Html, selector: &Selector) {
    let ids: Vec<_> = document.select(selector).map(|e| e.id()).collect();
    for id in ids {
        if let Some(mut node) = document.tree.get_mut(id) {
            node.detach();
        }
    }
}

fn unwrap_selected(document: &mut Html, selector: &Selector) {
    let ids: Vec<_> = document.select(selector).map(|e| e.id()).collect();
    for id in ids {
        let child_ids: Vec<_> = document
            .tree
            .get(id)
            .into_iter()
            .flat_map(|n| n.children().map(|c| c.id()))
            .collect();

        let Some(mut target) = document.tree.get_mut(id) else {
            continue;
        };
        for child_id in child_ids.into_iter().rev() {
            let _ = target.insert_id_before(child_id);
        }
        target.detach();
    }
}

fn extract_metadata(document: &Html, options: &ConversionOptions) -> ExtractedMetadata {
    let mut meta = ExtractedMetadata::default();
    if options.cleanup.metadata.title {
        meta.title = document
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string());
    }
    if options.cleanup.metadata.description {
        meta.description = select_meta(document, "meta[name=\"description\"]", "content");
    }
    if options.cleanup.metadata.canonical_url {
        meta.canonical_url = select_meta(document, "link[rel=\"canonical\"]", "href");
    }
    if options.cleanup.metadata.open_graph_title {
        meta.open_graph_title = select_meta(document, "meta[property=\"og:title\"]", "content");
    }
    if options.cleanup.metadata.open_graph_description {
        meta.open_graph_description =
            select_meta(document, "meta[property=\"og:description\"]", "content");
    }
    if options.cleanup.metadata.open_graph_image {
        meta.open_graph_image = select_meta(document, "meta[property=\"og:image\"]", "content");
    }
    if options.cleanup.metadata.twitter_title {
        meta.twitter_title = select_meta(document, "meta[name=\"twitter:title\"]", "content");
    }
    if options.cleanup.metadata.twitter_description {
        meta.twitter_description =
            select_meta(document, "meta[name=\"twitter:description\"]", "content");
    }
    meta
}

fn select_meta(document: &Html, selector: &str, attr: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    document
        .select(&selector)
        .next()
        .and_then(|e| e.value().attr(attr))
        .map(|s| s.trim().to_string())
}

// --- Phase 2 semantic / media / image handling ---

fn apply_image_handling(document: &mut Html, options: &ConversionOptions) {
    if options.cleanup.image_mode == ImageMode::Skip {
        detach_by_tag(document, "img");
        return;
    }

    let Ok(img_sel) = Selector::parse("img") else {
        return;
    };
    let ids: Vec<NodeId> = document.select(&img_sel).map(|e| e.id()).collect();

    for id in ids {
        promote_lazy_image(document, id, options);
        promote_srcset(document, id, options);
        if options.cleanup.preserve_image_metadata {
            append_image_metadata(document, id);
        }
        match options.cleanup.image_mode {
            ImageMode::AltText => {
                let alt = image_alt(document, id);
                replace_with_text(document, id, &alt);
            }
            ImageMode::Reference => {
                // Phase 2 fallback: htmd already produces inline images by default.
            }
            _ => {}
        }
    }
}

fn extract_language(class_value: &str, patterns: &[regex::Regex]) -> Option<String> {
    for re in patterns {
        if let Some(caps) = re.captures(class_value) {
            if let Some(m) = caps.name("lang").or_else(|| caps.get(1)) {
                let lang = m.as_str().to_string();
                if !lang.is_empty() {
                    return Some(lang);
                }
            }
        }
    }
    None
}

fn promote_lazy_image(document: &mut Html, id: NodeId, options: &ConversionOptions) {
    let Some(mut node) = document.tree.get_mut(id) else {
        return;
    };
    let Node::Element(ref mut el) = *node.value() else {
        return;
    };

    let has_src = el
        .attrs
        .iter()
        .any(|(name, _)| name.local.as_ref() == "src");
    if has_src {
        return;
    }

    for attr in &options.cleanup.lazy_image_attrs {
        let lazy_value = el
            .attrs
            .iter()
            .find(|(name, _)| name.local.as_ref() == attr)
            .map(|(_, v)| v.to_string());
        if let Some(value) = lazy_value {
            set_attr(el, "src", &value);
            break;
        }
    }
}

fn promote_srcset(document: &mut Html, id: NodeId, options: &ConversionOptions) {
    let (has_src, srcset) = {
        let Some(node) = document.tree.get(id) else {
            return;
        };
        let Node::Element(ref el) = *node.value() else {
            return;
        };
        let has_src = el
            .attrs
            .iter()
            .any(|(name, _)| name.local.as_ref() == "src");
        let srcset = el
            .attrs
            .iter()
            .find(|(name, _)| name.local.as_ref() == "srcset")
            .map(|(_, v)| v.to_string());
        (has_src, srcset)
    };
    if has_src {
        return;
    }
    let Some(srcset) = srcset else {
        return;
    };

    let url = match options.cleanup.responsive_image_policy {
        crate::options::ResponsiveImagePolicy::FirstSrcset
        | crate::options::ResponsiveImagePolicy::PreserveSrcset => srcset
            .split(',')
            .next()
            .and_then(|s| s.split_whitespace().next())
            .map(|s| s.to_string()),
        crate::options::ResponsiveImagePolicy::Largest => {
            let candidates: Vec<&str> = srcset.split(',').map(|s| s.trim()).collect();
            choose_largest_srcset(&candidates)
                .split_whitespace()
                .next()
                .map(|s| s.to_string())
        }
    };
    if let Some(url) = url {
        let Some(mut node) = document.tree.get_mut(id) else {
            return;
        };
        let Node::Element(ref mut el) = *node.value() else {
            return;
        };
        set_attr(el, "src", &url);
    }
}

fn append_image_metadata(document: &mut Html, id: NodeId) {
    let (width, height) = {
        let Some(node) = document.tree.get(id) else {
            return;
        };
        let Node::Element(ref el) = *node.value() else {
            return;
        };
        let w = el
            .attrs
            .iter()
            .find(|(n, _)| n.local.as_ref() == "width")
            .map(|(_, v)| v.to_string());
        let h = el
            .attrs
            .iter()
            .find(|(n, _)| n.local.as_ref() == "height")
            .map(|(_, v)| v.to_string());
        (w, h)
    };
    if width.is_none() && height.is_none() {
        return;
    }
    let meta = format!(
        "({}x{})",
        width.as_deref().unwrap_or("?"),
        height.as_deref().unwrap_or("?")
    );

    let Some(mut node) = document.tree.get_mut(id) else {
        return;
    };
    let Node::Element(ref mut el) = *node.value() else {
        return;
    };
    let current = el
        .attrs
        .iter()
        .find(|(n, _)| n.local.as_ref() == "alt")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    let new_alt = if current.is_empty() {
        meta
    } else {
        format!("{current} {meta}")
    };
    set_attr(el, "alt", &new_alt);
}

fn image_alt(document: &Html, id: NodeId) -> String {
    document
        .tree
        .get(id)
        .and_then(|n| n.value().as_element())
        .and_then(|el| el.attrs.iter().find(|(n, _)| n.local.as_ref() == "alt"))
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

fn apply_media_policy(document: &mut Html, options: &ConversionOptions) {
    match options.cleanup.media_policy {
        MediaPolicy::Drop => {
            for tag in ["video", "audio", "source", "figure", "figcaption"] {
                detach_by_tag(document, tag);
            }
        }
        MediaPolicy::Placeholder => {
            for tag in ["video", "audio"] {
                media_placeholder(document, tag);
            }
            unwrap_by_tag(document, "figure");
        }
        MediaPolicy::Inline | MediaPolicy::Link => {
            unwrap_by_tag(document, "figure");
        }
    }
}

fn media_placeholder(document: &mut Html, tag: &str) {
    let Ok(sel) = Selector::parse(tag) else {
        return;
    };
    let ids: Vec<NodeId> = document.select(&sel).map(|e| e.id()).collect();
    for id in ids {
        let src = document
            .tree
            .get(id)
            .and_then(|n| n.value().as_element())
            .and_then(|el| el.attrs.iter().find(|(n, _)| n.local.as_ref() == "src"))
            .map(|(_, v)| v.to_string());
        let label = tag.to_ascii_uppercase();
        let text = match src {
            Some(s) if !s.is_empty() => format!("({label}: {s})"),
            _ => format!("({label})"),
        };
        replace_with_text(document, id, &text);
    }
}

fn apply_details_handling(document: &mut Html, options: &ConversionOptions) {
    match options.cleanup.details_handling {
        DetailsHandling::Drop => {
            detach_by_tag(document, "details");
        }
        DetailsHandling::SummaryOnly => {
            let Ok(sel) = Selector::parse("details") else {
                return;
            };
            let ids: Vec<NodeId> = document.select(&sel).map(|e| e.id()).collect();
            for id in ids {
                let summary = first_summary_text(document, id);
                replace_with_text(document, id, &summary);
            }
        }
        DetailsHandling::Expand => {}
    }
}

fn first_summary_text(document: &Html, details_id: NodeId) -> String {
    let Some(details) = document.tree.get(details_id) else {
        return String::new();
    };
    for child in details.children() {
        if let Some(el) = child.value().as_element() {
            if el.name().eq_ignore_ascii_case("summary") {
                return node_text(&child).trim().to_string();
            }
        }
    }
    String::new()
}

fn apply_form_handling(document: &mut Html, options: &ConversionOptions) {
    match options.cleanup.form_handling {
        FormHandling::Drop => {
            detach_by_tag(document, "form");
        }
        FormHandling::Readable => {
            let Ok(sel) = Selector::parse("form") else {
                return;
            };
            let ids: Vec<NodeId> = document.select(&sel).map(|e| e.id()).collect();
            for id in ids {
                let text = form_readable_text(document, id);
                replace_with_text(document, id, &text);
            }
        }
        FormHandling::Checklist => {
            // Phase 3: checklist representation.
        }
    }
}

fn form_readable_text(document: &Html, form_id: NodeId) -> String {
    let Some(form) = document.tree.get(form_id) else {
        return String::new();
    };
    let mut lines = Vec::new();
    collect_form_lines(&form, &mut lines);
    lines.join("\n")
}

fn collect_form_lines(node: &ego_tree::NodeRef<'_, Node>, lines: &mut Vec<String>) {
    if let Some(el) = node.value().as_element() {
        let name = el.name().to_ascii_lowercase();
        if name == "input" || name == "button" || name == "select" || name == "textarea" {
            let value = input_value(el);
            let label = find_label(node).unwrap_or_default();
            if !label.is_empty() || !value.is_empty() {
                lines.push(format!("{label}: {value}").trim().to_string());
            }
            return;
        }
    }
    for child in node.children() {
        collect_form_lines(&child, lines);
    }
}

fn input_value(el: &scraper::node::Element) -> String {
    let tag = el.name().to_ascii_lowercase();
    let mut value = el
        .attrs
        .iter()
        .find(|(n, _)| n.local.as_ref() == "value")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    if value.is_empty() {
        value = el
            .attrs
            .iter()
            .find(|(n, _)| n.local.as_ref() == "placeholder")
            .map(|(_, v)| v.to_string())
            .unwrap_or_default();
    }
    if tag == "button" && value.is_empty() {
        // text content handled by traversal
    }
    value
}

fn find_label(node: &ego_tree::NodeRef<'_, Node>) -> Option<String> {
    // Search preceding siblings and ancestors for a <label> that references this input.
    let id = node
        .value()
        .as_element()
        .and_then(|el| el.attrs.iter().find(|(n, _)| n.local.as_ref() == "id"))
        .map(|(_, v)| v.to_string())?;

    // Simple heuristic: search the whole document fragment under the input's ancestor.
    let mut current = Some(*node);
    while let Some(n) = current {
        if let Some(parent) = n.parent() {
            for label in parent.descendants() {
                if let Some(el) = label.value().as_element() {
                    if el.name().eq_ignore_ascii_case("label") {
                        let references = el.attrs.iter().any(|(name, value)| {
                            (name.local.as_ref() == "for" && value.as_ref() == id)
                                || value.as_ref().contains(&id)
                        });
                        if references {
                            return Some(node_text(&label).trim().to_string());
                        }
                    }
                }
            }
        }
        current = n.parent();
    }
    None
}

fn apply_custom_elements(document: &mut Html, options: &ConversionOptions) {
    match options.cleanup.custom_element_policy {
        CustomElementPolicy::Unwrap => unwrap_hyphenated(document),
        CustomElementPolicy::Drop => drop_hyphenated(document),
        CustomElementPolicy::PreserveHtml => {}
    }
}

fn hyphenated_element_ids(document: &Html) -> Vec<NodeId> {
    document
        .tree
        .nodes()
        .filter(|n| {
            n.value()
                .as_element()
                .map(|el| {
                    let name = el.name();
                    name.contains('-') && !name.eq_ignore_ascii_case("annotation-xml")
                })
                .unwrap_or(false)
        })
        .map(|n| n.id())
        .collect()
}

fn unwrap_hyphenated(document: &mut Html) {
    let ids = hyphenated_element_ids(document);
    for id in ids {
        unwrap_node(document, id);
    }
}

fn drop_hyphenated(document: &mut Html) {
    let ids = hyphenated_element_ids(document);
    for id in ids {
        if let Some(mut node) = document.tree.get_mut(id) {
            node.detach();
        }
    }
}

fn apply_per_tag_behavior(document: &mut Html, options: &ConversionOptions) {
    for (tag, action) in &options.cleanup.per_tag_behavior {
        let Ok(sel) = Selector::parse(tag) else {
            continue;
        };
        match action.as_str() {
            "drop" => detach_selected(document, &sel),
            "unwrap" => unwrap_selected(document, &sel),
            "text" => {
                let ids: Vec<NodeId> = document.select(&sel).map(|e| e.id()).collect();
                for id in ids {
                    replace_with_inner_text(document, id);
                }
            }
            "html" | "preserve" => {}
            _ => {}
        }
    }
}

static SEL_HEADINGS: Lazy<Selector> = Lazy::new(|| Selector::parse("h1,h2,h3,h4,h5,h6").unwrap());

/// Shift heading levels by `semantic.heading_offset`, clamping the result to
/// the valid `h1`..`h6` range.
fn apply_heading_offset(document: &mut Html, options: &ConversionOptions) {
    let offset = i32::from(options.semantic.heading_offset);
    if offset == 0 {
        return;
    }

    let targets: Vec<(NodeId, i32)> = document
        .select(&SEL_HEADINGS)
        .filter_map(|e| {
            let level = e.value().name().strip_prefix('h')?.parse::<i32>().ok()?;
            Some((e.id(), level))
        })
        .collect();

    for (id, level) in targets {
        let new_level = (level + offset).clamp(1, 6);
        if new_level == level {
            continue;
        }
        let Some(mut node) = document.tree.get_mut(id) else {
            continue;
        };
        let Node::Element(ref mut el) = *node.value() else {
            continue;
        };
        el.name = QualName::new(
            None,
            el.name.ns.clone(),
            LocalName::from(format!("h{new_level}").as_str()),
        );
    }
}

// --- Phase 3: custom rules, mermaid, tables, language detection ---

fn apply_custom_rules(
    document: &mut Html,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    if options.extension.custom_rules.is_empty() {
        return Ok(());
    }

    let mut rules: Vec<_> = options.extension.custom_rules.iter().enumerate().collect();
    rules.sort_by(|a, b| b.1.priority.cmp(&a.1.priority).then_with(|| a.0.cmp(&b.0)));

    for (index, rule) in rules {
        for selector_str in &rule.selectors {
            let selector = match Selector::parse(selector_str) {
                Ok(s) => s,
                Err(e) => {
                    let mut d = Diagnostic::warning(format!(
                        "invalid custom rule selector {selector_str}: {e}"
                    ))
                    .with_rule(index.to_string());
                    if let Some(p) = path {
                        d = d.with_path(p);
                    }
                    diagnostics.push(d);
                    if options.strict {
                        return Err(Error::Selector(format!(
                            "invalid custom rule selector {selector_str}: {e}"
                        )));
                    }
                    continue;
                }
            };

            let ids: Vec<NodeId> = document.select(&selector).map(|e| e.id()).collect();
            for id in ids {
                match rule.action {
                    CustomRuleAction::Drop => {
                        if let Some(mut node) = document.tree.get_mut(id) {
                            node.detach();
                        }
                    }
                    CustomRuleAction::Unwrap => unwrap_node(document, id),
                    CustomRuleAction::Text => replace_with_inner_text(document, id),
                    CustomRuleAction::Html => {}
                    CustomRuleAction::MarkdownTemplate
                    | CustomRuleAction::FencedBlock
                    | CustomRuleAction::Link
                    | CustomRuleAction::Image => {
                        // These actions produce raw Markdown syntax. Matched
                        // elements are tagged later by
                        // `apply_custom_rule_markers` (after attribute
                        // normalization) and rendered by the backend handler,
                        // so the output is not escaped by the Markdown text
                        // formatter.
                    }
                }
            }
        }
    }

    Ok(())
}

/// Tag elements matched by Markdown-producing custom rules (template, fenced
/// block, link, image) with the `htmlmdrule` marker tag and the rule's index,
/// which the backend's single custom-rule handler resolves. This gives those
/// actions full CSS-selector support and one priority order (descending,
/// declaration order as tiebreak — the first matching rule claims the
/// element). Runs late in the pipeline so earlier passes (lazy-image
/// promotion, language detection) have already normalized attributes.
fn apply_custom_rule_markers(
    document: &mut Html,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<()> {
    let mut rules: Vec<_> = options
        .extension
        .custom_rules
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            matches!(
                r.action,
                CustomRuleAction::MarkdownTemplate
                    | CustomRuleAction::FencedBlock
                    | CustomRuleAction::Link
                    | CustomRuleAction::Image
            )
        })
        .collect();
    if rules.is_empty() {
        return Ok(());
    }
    rules.sort_by(|a, b| b.1.priority.cmp(&a.1.priority).then_with(|| a.0.cmp(&b.0)));

    for (index, rule) in rules {
        for selector_str in &rule.selectors {
            let selector = match Selector::parse(selector_str) {
                Ok(s) => s,
                Err(e) => {
                    let mut d = Diagnostic::warning(format!(
                        "invalid custom rule selector {selector_str}: {e}"
                    ))
                    .with_rule(index.to_string());
                    if let Some(p) = path {
                        d = d.with_path(p);
                    }
                    diagnostics.push(d);
                    if options.strict {
                        return Err(Error::Selector(format!(
                            "invalid custom rule selector {selector_str}: {e}"
                        )));
                    }
                    continue;
                }
            };

            let ids: Vec<NodeId> = document.select(&selector).map(|e| e.id()).collect();
            for id in ids {
                let Some(mut node) = document.tree.get_mut(id) else {
                    continue;
                };
                let Node::Element(ref mut el) = *node.value() else {
                    continue;
                };
                // First (highest-priority) matching rule claims the element.
                if el
                    .attrs
                    .iter()
                    .any(|(n, _)| n.local.as_ref() == "data-htmlmd-rule")
                {
                    continue;
                }
                set_attr(el, "data-htmlmd-rule", &index.to_string());
                el.name = QualName::new(None, el.name.ns.clone(), LocalName::from("htmlmdrule"));
            }
        }
    }
    Ok(())
}

fn apply_table_handling(document: &mut Html, options: &ConversionOptions) {
    let ids: Vec<NodeId> = document.select(&SEL_TABLE).map(|e| e.id()).collect();

    for id in ids {
        let is_complex = is_complex_table(document, id);
        let handling = match options.semantic.table_handling {
            TableHandling::Gfm if is_complex => match options.semantic.difficult_table_strategy {
                DifficultTableStrategy::HtmlFallback => TableHandling::HtmlFallback,
                _ => TableHandling::Flatten,
            },
            TableHandling::Gfm => TableHandling::Gfm,
            other => other,
        };

        match handling {
            TableHandling::Gfm => {}
            TableHandling::HtmlFallback => html_fallback_table(document, id),
            TableHandling::Flatten => flatten_table(document, id),
            TableHandling::CsvLike => csv_like_table(document, id),
            TableHandling::Drop => {
                if let Some(mut node) = document.tree.get_mut(id) {
                    node.detach();
                }
            }
        }
    }
}

fn is_complex_table(document: &Html, id: NodeId) -> bool {
    let Some(el) = document.tree.get(id).and_then(scraper::ElementRef::wrap) else {
        return false;
    };
    // Nested tables are always complex.
    if el.select(&SEL_NESTED_TABLE).next().is_some() {
        return true;
    }

    let mut row_lengths = Vec::new();
    for row in el.select(&SEL_TR) {
        let cells = row.select(&SEL_CELLS).count();
        let has_span = row.select(&SEL_SPAN_CELLS).next().is_some();
        if has_span {
            return true;
        }
        row_lengths.push(cells);
    }

    if row_lengths.len() < 2 {
        return false;
    }
    let first = row_lengths[0];
    row_lengths.iter().any(|&len| len != first || len == 0)
}

fn flatten_table(document: &mut Html, id: NodeId) {
    let text = document
        .tree
        .get(id)
        .and_then(scraper::ElementRef::wrap)
        .map(|el| {
            let mut lines = Vec::new();
            for row in el.select(&SEL_TR) {
                let cells: Vec<String> = row
                    .select(&SEL_CELLS)
                    .map(|c| c.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !cells.is_empty() {
                    lines.push(cells.join(" | "));
                }
            }
            lines.join("\n\n")
        })
        .unwrap_or_default();

    if text.is_empty() {
        if let Some(mut node) = document.tree.get_mut(id) {
            node.detach();
        }
    } else {
        replace_with_text(document, id, &format!("\n\n{text}\n\n"));
    }
}

fn csv_like_table(document: &mut Html, id: NodeId) {
    let Some(mut node) = document.tree.get_mut(id) else {
        return;
    };
    let Node::Element(ref mut el) = *node.value() else {
        return;
    };
    set_attr(el, "data-htmlmd-table", "csv");
}

fn html_fallback_table(document: &mut Html, id: NodeId) {
    let Some(mut node) = document.tree.get_mut(id) else {
        return;
    };
    let Node::Element(ref mut el) = *node.value() else {
        return;
    };
    set_attr(el, "data-htmlmd-table", "html");
}

fn apply_code_language_detection(document: &mut Html, options: &ConversionOptions) {
    if options.semantic.code_language_patterns.is_empty() && !options.semantic.detect_languages {
        return;
    }

    let Ok(code_sel) = Selector::parse("pre code, code") else {
        return;
    };

    // User-configured patterns come from the process-wide cache, so their
    // compile cost is paid once per process, not per element or conversion.
    let patterns: Vec<regex::Regex> = options
        .semantic
        .code_language_patterns
        .iter()
        .filter_map(|p| crate::regex_cache::cached_regex(p))
        .collect();

    let ids: Vec<NodeId> = document.select(&code_sel).map(|e| e.id()).collect();
    for id in ids {
        let pre_class = document
            .tree
            .get(id)
            .and_then(|n| n.parent())
            .and_then(|p| p.value().as_element())
            .filter(|el| el.name().eq_ignore_ascii_case("pre"))
            .and_then(|el| el.attrs.iter().find(|(n, _)| n.local.as_ref() == "class"))
            .map(|(_, v)| v.to_string());

        let code_class = document
            .tree
            .get(id)
            .and_then(|n| n.value().as_element())
            .and_then(|el| el.attrs.iter().find(|(n, _)| n.local.as_ref() == "class"))
            .map(|(_, v)| v.to_string());

        let source = code_class.as_deref().unwrap_or("");
        let mut lang = extract_language(source, &patterns)
            .or_else(|| extract_language(pre_class.as_deref().unwrap_or(""), &patterns));

        if lang.is_none() && options.semantic.detect_languages {
            let text = code_text(document, id);
            lang = detect_language_from_code(&text).map(|s| s.to_string());
        }

        if let Some(lang) = lang {
            let Some(mut node) = document.tree.get_mut(id) else {
                continue;
            };
            let Node::Element(ref mut el) = *node.value() else {
                continue;
            };
            set_attr(el, "class", &format!("language-{lang}"));
        }
    }
}

fn code_text(document: &Html, id: NodeId) -> String {
    document
        .tree
        .get(id)
        .map(|n| node_text(&n))
        .unwrap_or_default()
}

fn detect_language_from_code(text: &str) -> Option<&'static str> {
    use once_cell::sync::Lazy;
    use regex::Regex;

    static RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
        vec![
            (Regex::new(r#"(?m)^\s*<\?php\b"#).unwrap(), "php"),
            (
                Regex::new(r#"(?m)^\s*#!\s*/usr/bin/env\s+(?:python3?|python)\b"#).unwrap(),
                "python",
            ),
            (
                Regex::new(r#"(?m)^\s*#!\s*/usr/bin/python"#).unwrap(),
                "python",
            ),
            (
                Regex::new(r#"(?m)^\s*#!\s*/usr/bin/env\s+(?:node|nodejs)\b"#).unwrap(),
                "javascript",
            ),
            (
                Regex::new(r#"(?m)^\s*#!\s*/usr/bin/env\s+ruby\b"#).unwrap(),
                "ruby",
            ),
            (
                Regex::new(r#"(?m)^\s*#!\s*/usr/bin/env\s+perl\b"#).unwrap(),
                "perl",
            ),
            (
                Regex::new(r#"(?m)^\s*#!\s*/usr/bin/env\s+lua\b"#).unwrap(),
                "lua",
            ),
            (
                Regex::new(r#"(?m)^\s*#!\s*/bin/(?:bash|sh)\b"#).unwrap(),
                "shell",
            ),
            (Regex::new(r#"(?m)^\s*#!\s*/bin/zsh\b"#).unwrap(), "shell"),
            (Regex::new(r#"(?m)^\s*<!DOCTYPE\s+html\b"#).unwrap(), "html"),
            (Regex::new(r#"(?m)^\s*<\?xml\b"#).unwrap(), "xml"),
            (Regex::new(r#"(?m)^\s*<html\b"#).unwrap(), "html"),
            (Regex::new(r#"(?m)^\s*package\s+main\s*$"#).unwrap(), "go"),
            (Regex::new(r#"(?m)^\s*func\s+\w+\s*\("#).unwrap(), "go"),
            (Regex::new(r#"(?m)^\s*fn\s+\w+\s*\("#).unwrap(), "rust"),
            (
                Regex::new(r#"(?m)^\s*pub\s+fn\s+\w+\s*\("#).unwrap(),
                "rust",
            ),
            (
                Regex::new(r#"(?m)^\s*#include\s*[<\"][^>\"]*\.hpp[>\"]|std::"#).unwrap(),
                "cpp",
            ),
            (Regex::new(r#"(?m)^\s*#include\s*[<\"]"#).unwrap(), "c"),
            (Regex::new(r#"(?m)^\s*def\s+\w+\s*\("#).unwrap(), "python"),
            (
                Regex::new(r#"(?m)^\s*class\s+\w+\s*\{[^}]*System\.out"#).unwrap(),
                "java",
            ),
            (
                Regex::new(r#"(?m)^\s*console\.log\("#).unwrap(),
                "javascript",
            ),
            (Regex::new(r#"(?m)^\s*document\."#).unwrap(), "javascript"),
            (
                Regex::new(r#"(?m)^\s*SELECT\s+[\w*]+\s+FROM\s+\w+"#).unwrap(),
                "sql",
            ),
            (Regex::new(r#"(?m)^\s*\$\("#).unwrap(), "shell"),
            (Regex::new(r#"(?m)^\s*echo\s+"#).unwrap(), "shell"),
            (Regex::new(r#"(?m)^\s*using\s+System"#).unwrap(), "cs"),
            (Regex::new(r#"(?m)^\s*@import\s+"#).unwrap(), "css"),
            (Regex::new(r#"(?m)^\s*\{\s*[\"']"#).unwrap(), "json"),
            (Regex::new(r#"(?m)^\s*\[\s*[\"']"#).unwrap(), "json"),
            (Regex::new(r#"(?m)^\s*\{\s*\}\s*$"#).unwrap(), "json"),
            (Regex::new(r#"(?m)^\s*<\w+[^>]*>"#).unwrap(), "html"),
        ]
    });

    for (re, lang) in RULES.iter() {
        if re.is_match(text) {
            return Some(lang);
        }
    }
    None
}

// --- low-level DOM helpers ---

fn replace_with_text(document: &mut Html, id: NodeId, text: &str) {
    let Some(mut target) = document.tree.get_mut(id) else {
        return;
    };
    let text_node = Node::Text(scraper::node::Text {
        text: text.to_string().into(),
    });
    target.insert_before(text_node);
    target.detach();
}

fn replace_with_inner_text(document: &mut Html, id: NodeId) {
    let text = document
        .tree
        .get(id)
        .map(|n| node_text(&n).trim().to_string())
        .unwrap_or_default();
    replace_with_text(document, id, &text);
}

fn node_text(node: &NodeRef<'_, Node>) -> String {
    let mut out = String::new();
    collect_text(node, &mut out);
    out
}

fn collect_text(node: &NodeRef<'_, Node>, out: &mut String) {
    if let Some(text) = node.value().as_text() {
        out.push_str(text);
    } else {
        for child in node.children() {
            collect_text(&child, out);
        }
    }
}

fn detach_by_tag(document: &mut Html, tag: &str) {
    if let Ok(sel) = Selector::parse(tag) {
        detach_selected(document, &sel);
    }
}

fn unwrap_by_tag(document: &mut Html, tag: &str) {
    if let Ok(sel) = Selector::parse(tag) {
        unwrap_selected(document, &sel);
    }
}

fn unwrap_node(document: &mut Html, id: NodeId) {
    let child_ids: Vec<NodeId> = document
        .tree
        .get(id)
        .into_iter()
        .flat_map(|n| n.children().map(|c| c.id()))
        .collect();

    let Some(mut target) = document.tree.get_mut(id) else {
        return;
    };
    for child_id in child_ids.into_iter().rev() {
        let _ = target.insert_id_before(child_id);
    }
    target.detach();
}

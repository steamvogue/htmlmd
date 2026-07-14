// SPDX-License-Identifier: MIT OR Apache-2.0

use ego_tree::{NodeId, NodeRef};
use markup5ever::{LocalName, QualName};
use scraper::{Html, Node, Selector};

use crate::diagnostic::{Diagnostic, DiagnosticsCollector};
use crate::error::{Error, Result};
use crate::options::{
    ConversionOptions, CustomElementPolicy, DetailsHandling, FormHandling, HiddenContentPolicy,
    ImageMode, MediaPolicy, TitleHandling,
};
use crate::rewrite::rewrite_url_attr;

/// Apply HTML cleanup, content selection, and URL rewriting to a string of HTML.
///
/// Returns the cleaned HTML string and extracted metadata.
pub fn clean_html(
    html: &str,
    options: &ConversionOptions,
    path: Option<&str>,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> Result<(String, ExtractedMetadata)> {
    let mut document = Html::parse_document(html);

    check_limits(html, &document, options, diagnostics)?;

    // Extract metadata from the original document before any destructive cleanup.
    let metadata = extract_metadata(&document, options);

    remove_hidden(&mut document, options);
    apply_remove_tags(&mut document, options);
    apply_per_tag_behavior(&mut document, options);
    apply_remove_selectors(&mut document, options, path, diagnostics)?;
    apply_unwrap_selectors(&mut document, options, path, diagnostics)?;
    apply_keep_only(&mut document, options, path, diagnostics)?;
    apply_details_handling(&mut document, options);
    apply_form_handling(&mut document, options);
    apply_custom_elements(&mut document, options);
    apply_media_policy(&mut document, options);
    apply_image_handling(&mut document, options);
    apply_url_rewriting(&mut document, options, diagnostics)?;

    if options.render.title_attribute == TitleHandling::Ignore {
        strip_attribute(&mut document, "title");
    }

    let cleaned = document.html();
    Ok((cleaned, metadata))
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
            let msg = format!("DOM node count {count} exceeds limit {}", limits.max_node_count);
            if options.strict {
                return Err(Error::LimitExceeded(msg));
            }
            diagnostics.push(Diagnostic::warning(msg));
        }
    }
    Ok(())
}

fn apply_remove_tags(document: &mut Html, options: &ConversionOptions) {
    for tag in &options.cleanup.remove_tags {
        let Ok(selector) = Selector::parse(tag) else {
            continue;
        };
        detach_selected(document, &selector);
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
            let rewritten = rewrite_url_attr(&original, attr_name, base.as_ref(), options, diagnostics);
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

fn promote_lazy_image(document: &mut Html, id: NodeId, options: &ConversionOptions) {
    let Some(mut node) = document.tree.get_mut(id) else {
        return;
    };
    let Node::Element(ref mut el) = *node.value() else {
        return;
    };

    let has_src = el.attrs.iter().any(|(name, _)| name.local.as_ref() == "src");
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
        let has_src = el.attrs.iter().any(|(name, _)| name.local.as_ref() == "src");
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
        crate::options::ResponsiveImagePolicy::FirstSrcset | crate::options::ResponsiveImagePolicy::PreserveSrcset => {
            srcset.split(',').next().and_then(|s| s.split_whitespace().next()).map(|s| s.to_string())
        }
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

fn choose_largest_srcset<'a>(candidates: &'a [&'a str]) -> &'a str {
    candidates
        .iter()
        .max_by_key(|c| {
            c.split_whitespace()
                .nth(1)
                .and_then(|d| {
                    if let Some(n) = d.strip_suffix('w') {
                        n.parse::<u32>().ok()
                    } else {
                        d.strip_suffix('x').map(|n| (n.parse::<f32>().unwrap_or(0.0) * 1000.0) as u32)
                    }
                })
                .unwrap_or(0)
        })
        .copied()
        .unwrap_or(candidates[0])
}

fn append_image_metadata(document: &mut Html, id: NodeId) {
    let (width, height) = {
        let Some(node) = document.tree.get(id) else {
            return;
        };
        let Node::Element(ref el) = *node.value() else {
            return;
        };
        let w = el.attrs.iter().find(|(n, _)| n.local.as_ref() == "width").map(|(_, v)| v.to_string());
        let h = el.attrs.iter().find(|(n, _)| n.local.as_ref() == "height").map(|(_, v)| v.to_string());
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

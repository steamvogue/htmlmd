// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

//! Native HTML-to-Markdown renderer: htmd v0.5.4's rendering core ported from
//! `markup5ever_rcdom` to scraper's `ego_tree`, so an already-parsed
//! (cleaned) `scraper::Html` document can be rendered without a second parse.
//!
//! Layout mirrors htmd's `src/` so Phase B can port the custom handlers in
//! `htmd_handlers.rs` mechanically:
//!
//! | htmd v0.5.4          | this module                                    |
//! |----------------------|------------------------------------------------|
//! | `lib.rs`             | `mod.rs` (`Element`, converter, builder)        |
//! | `dom_walker.rs`      | `dom_walker.rs`                                 |
//! | `text_util.rs`       | `text_util.rs`                                  |
//! | `html_escape.rs`     | `html_escape.rs`                                |
//! | `node_util.rs`       | `node_util.rs`                                  |
//! | `options.rs`         | `options.rs`                                    |
//! | `element_handler/*`  | `element_handler/*`                             |
//!
//! Node mapping: `&Rc<Node>` → `ego_tree::NodeRef<'_, scraper::Node>` (Copy,
//! by value), `node.children.borrow()` → `node.children()`, `Weak` parent
//! links → `node.parent()`, `&[Attribute]` → `&[(QualName, StrTendril)]`.
//! htmd's one DOM *mutation* (combining similar adjacent inline elements in
//! `walk_children`) is replicated with a side table; see `dom_walker`.

pub(crate) mod dom_walker;
pub(crate) mod element_handler;
mod html_escape;
mod marker_handlers;
pub(crate) mod node_util;
pub(crate) mod options;
pub(crate) mod text_util;

use ego_tree::NodeRef;
use markup5ever::QualName;
use scraper::{Html, Node, StrTendril};

use dom_walker::walk_node;
use element_handler::{ElementHandler, ElementHandlers, Handlers};
use options::Options;

use crate::backend::ConverterBackend;
use crate::error::Result;
use crate::options::{
    BulletMarker, CodeFence, ConversionOptions, HardBreakStyle, HeadingStyle,
    HrStyle as HtmlMdHrStyle, LinkStyle as HtmlMdLinkStyle, MermaidPolicy, RawHtmlPolicy,
    ReferencePlacement,
};
use crate::result::ConversionResult;

/// The DOM element handed to element handlers (htmd's `Element`, with rcdom
/// types replaced by scraper/ego-tree types).
pub(crate) struct Element<'a> {
    /// The ego-tree node of the element.
    pub node: NodeRef<'a, Node>,
    /// The tag name.
    pub tag: &'a str,
    /// The attribute list.
    pub attrs: &'a [(QualName, StrTendril)],
    /// When true, this element's children were all translated using Markdown,
    /// not HTML. This is only needed in faithful translation mode (see the
    /// `Options`): for code blocks, translating a `<pre><code>` sequence to
    /// Markdown, not HTML, requires a Markdown translated `<code>` block;
    /// likewise, translating lists ((`<ol>`/`<ul>`)`<li>`) to Markdown requires
    /// all `<li>` elements are translated to Markdown.
    pub markdown_translated: bool,
    /// The number of handlers to skip for this element.
    pub(crate) skipped_handlers: usize,
}

/// The native html-to-markdown converter (htmd's `HtmlToMarkdown` over
/// ego-tree). Built per conversion by `NativeBackend`.
pub(crate) struct NativeConverter {
    handlers: ElementHandlers,
}

impl NativeConverter {
    /// Create a new [NativeConverterBuilder].
    pub(crate) fn builder() -> NativeConverterBuilder {
        NativeConverterBuilder::new()
    }

    /// Convert an already-parsed DOM tree to Markdown (htmd's
    /// `tree_to_markdown`).
    pub(crate) fn dom_to_markdown(&self, document: &Html) -> String {
        // ego-tree `NodeId`s are per-tree indexes, so stale entries from a
        // previous document could alias nodes of this one. The backend builds
        // a fresh converter per conversion; this is belt-and-braces.
        self.handlers.combined_texts.borrow_mut().clear();

        let mut content = String::new();

        walk_node(
            document.tree.root(),
            &mut content,
            &self.handlers,
            None,
            true,
            false,
        );

        let mut content = content.trim_matches(|ch| ch == '\n').to_string();

        let mut append = String::new();
        for handler in &self.handlers.handlers {
            let Some(append_content) = handler.append() else {
                continue;
            };
            append.push_str(&append_content);
        }

        content.push_str(append.trim_end_matches('\n'));

        content
    }
}

/// The [NativeConverter] builder (htmd's `HtmlToMarkdownBuilder`).
///
/// htmd's builder also has a `scripting_enabled` knob; it only affects
/// html5ever *parsing* (whether `<noscript>` content becomes raw text or DOM).
/// The native pipeline's parses happen in scraper (during cleanup, or in
/// `NativeBackend::convert`), which hardcodes html5ever's default
/// `scripting_enabled = true`, and the walker itself does nothing
/// noscript-specific — so there is no knob to port here.
pub(crate) struct NativeConverterBuilder {
    handlers: ElementHandlers,
}

impl NativeConverterBuilder {
    /// Create a new builder.
    pub(crate) fn new() -> Self {
        let options = Options::default();
        let handlers = ElementHandlers::new(options);
        Self { handlers }
    }

    /// Set converting options.
    pub(crate) fn options(mut self, options: Options) -> Self {
        self.handlers.options = options;
        self
    }

    /// Skip a group of tags when converting.
    pub(crate) fn skip_tags(self, tags: Vec<&str>) -> Self {
        self.add_handler(tags, |_: &dyn Handlers, _: Element| None)
    }

    /// Apply a custom element handler for a group of tags.
    pub(crate) fn add_handler<Handler>(mut self, tags: Vec<&str>, handler: Handler) -> Self
    where
        Handler: ElementHandler + 'static,
    {
        self.handlers.add_handler(tags, handler);
        self
    }

    /// Create a new [NativeConverter].
    pub(crate) fn build(self) -> NativeConverter {
        NativeConverter {
            handlers: self.handlers,
        }
    }
}

/// The native converter backend: renders Markdown directly from scraper's DOM
/// tree, so a cleaned document is parsed exactly once.
///
/// Behavior contract: byte-identical output to [`crate::HtmdBackend`]
/// (enforced by `tests/native_parity.rs`).
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeBackend;

impl NativeBackend {
    pub fn new() -> Self {
        Self
    }
}

impl ConverterBackend for NativeBackend {
    fn convert(&self, html: &str, options: &ConversionOptions) -> Result<ConversionResult> {
        self.convert_dom(&Html::parse_document(html), options)
    }

    fn convert_dom(
        &self,
        document: &Html,
        options: &ConversionOptions,
    ) -> Result<ConversionResult> {
        let converter = build_native_converter(options);
        let markdown = converter.dom_to_markdown(document);
        let markdown = crate::htmd_backend::post_process(&markdown, options);

        Ok(ConversionResult {
            markdown,
            title: None,
            description: None,
            canonical_url: None,
            diagnostics: Vec::new(),
        })
    }
}

/// Build a fully configured converter from `ConversionOptions`.
///
/// Mirrors `htmd_handlers::build_converter` minus the custom handlers that
/// are Phase B work; the marker handlers registered here are the ones active
/// with default options (see `marker_handlers`).
fn build_native_converter(options: &ConversionOptions) -> NativeConverter {
    let native_options = build_native_options(options);

    let mut builder = NativeConverter::builder().options(native_options);

    let skip_tags: Vec<&str> = options
        .cleanup
        .remove_tags
        .iter()
        .map(|s| s.as_str())
        .collect();
    if !skip_tags.is_empty() {
        builder = builder.skip_tags(skip_tags);
    }

    // Registration order matches `htmd_handlers::build_converter`: task
    // lists, then table markers, then mermaid.
    if options.semantic.task_lists {
        builder = builder.add_handler(vec!["input"], marker_handlers::task_list_checkbox_handler);
    }
    builder = builder.add_handler(vec!["table"], marker_handlers::table_marker_handler);
    match options.semantic.mermaid {
        MermaidPolicy::Drop => {
            builder = builder
                .add_handler(vec!["pre"], marker_handlers::mermaid_drop_handler)
                .add_handler(vec!["div"], marker_handlers::mermaid_drop_handler);
        }
        MermaidPolicy::Fenced => {
            builder = builder
                .add_handler(vec!["pre"], marker_handlers::mermaid_fenced_handler)
                .add_handler(vec!["div"], marker_handlers::mermaid_fenced_handler);
        }
        MermaidPolicy::PreserveHtml => {}
    }

    builder.build()
}

/// Map `ConversionOptions` to the native `Options`.
///
/// This is a copy of `htmd_handlers::build_htmd_options` targeting the ported
/// `Options` type instead of htmd's (the mapping logic must stay in lockstep;
/// it cannot be shared because that function returns `htmd` types).
#[allow(clippy::field_reassign_with_default)]
fn build_native_options(options: &ConversionOptions) -> Options {
    let mut o = Options::default();

    o.heading_style = match options.render.heading_style {
        HeadingStyle::Atx | HeadingStyle::Keep => options::HeadingStyle::Atx,
        HeadingStyle::Setex => options::HeadingStyle::Setex,
    };

    o.hr_style = match options.render.hr_style {
        HtmlMdHrStyle::Dashes => options::HrStyle::Dashes,
        HtmlMdHrStyle::Asterisks => options::HrStyle::Asterisks,
        HtmlMdHrStyle::Underscores => options::HrStyle::Underscores,
    };

    o.br_style = match options.render.hard_break_style {
        HardBreakStyle::TwoSpaces => options::BrStyle::TwoSpaces,
        HardBreakStyle::Backslash => options::BrStyle::Backslash,
    };

    o.bullet_list_marker = match options.render.bullet_marker {
        BulletMarker::Asterisk => options::BulletListMarker::Asterisk,
        BulletMarker::Hyphen | BulletMarker::Plus => options::BulletListMarker::Dash,
    };

    o.code_block_fence = match options.render.code_fence {
        CodeFence::Backticks => options::CodeBlockFence::Backticks,
        CodeFence::Tildes => options::CodeBlockFence::Tildes,
    };

    o.link_style = match options.render.link_style {
        HtmlMdLinkStyle::Inline => options::LinkStyle::Inlined,
        HtmlMdLinkStyle::Reference
        | HtmlMdLinkStyle::CollapsedReference
        | HtmlMdLinkStyle::ShortcutReference => options::LinkStyle::Referenced,
    };

    o.link_reference_style = match options.render.reference_placement {
        ReferencePlacement::End | ReferencePlacement::SectionEnd => {
            options::LinkReferenceStyle::Full
        }
        ReferencePlacement::Adjacent => options::LinkReferenceStyle::Full,
    };

    match options.render.link_style {
        HtmlMdLinkStyle::CollapsedReference => {
            o.link_reference_style = options::LinkReferenceStyle::Collapsed;
        }
        HtmlMdLinkStyle::ShortcutReference => {
            o.link_reference_style = options::LinkReferenceStyle::Shortcut;
        }
        _ => {}
    }

    o.translation_mode = match options.render.raw_html_policy {
        RawHtmlPolicy::Faithful => options::TranslationMode::Faithful,
        _ => options::TranslationMode::Pure,
    };

    o
}

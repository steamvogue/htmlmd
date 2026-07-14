// SPDX-License-Identifier: MIT OR Apache-2.0

use htmd::options::{
    BrStyle, BulletListMarker, CodeBlockFence, HeadingStyle, HrStyle, LinkReferenceStyle,
    LinkStyle, Options as HtmdOptions, TranslationMode,
};
use htmd::HtmlToMarkdown;

use crate::backend::ConverterBackend;
use crate::error::{Error, Result};
use crate::options::{
    BulletMarker, CodeFence, ConversionOptions, FinalNewlinePolicy, HardBreakStyle,
    HeadingStyle as HtmlMdHeadingStyle, HrStyle as HtmlMdHrStyle, LinkStyle as HtmlMdLinkStyle,
    RawHtmlPolicy, ReferencePlacement, WhitespacePolicy,
};
use crate::result::ConversionResult;

/// The `htmd`-based converter backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct HtmdBackend;

impl HtmdBackend {
    pub fn new() -> Self {
        Self
    }
}

impl ConverterBackend for HtmdBackend {
    fn convert(&self, html: &str, options: &ConversionOptions) -> Result<ConversionResult> {
        let htmd_options = build_htmd_options(options);
        let scripting_enabled = options.cleanup.remove_tags.iter().all(|t| t != "noscript");

        let mut builder = HtmlToMarkdown::builder()
            .options(htmd_options)
            .scripting_enabled(scripting_enabled);

        let skip_tags: Vec<&str> = options.cleanup.remove_tags.iter().map(|s| s.as_str()).collect();
        if !skip_tags.is_empty() {
            builder = builder.skip_tags(skip_tags);
        }

        let markdown = builder
            .build()
            .convert(html)
            .map_err(|e| Error::Parse(e.to_string()))?;
        let markdown = post_process(&markdown, options);

        Ok(ConversionResult {
            markdown,
            title: None,
            description: None,
            canonical_url: None,
            diagnostics: Vec::new(),
        })
    }
}

#[allow(clippy::field_reassign_with_default)]
fn build_htmd_options(options: &ConversionOptions) -> HtmdOptions {
    let mut o = HtmdOptions::default();

    o.heading_style = match options.render.heading_style {
        HtmlMdHeadingStyle::Atx => HeadingStyle::Atx,
        HtmlMdHeadingStyle::Setex => HeadingStyle::Setex,
        HtmlMdHeadingStyle::Keep => HeadingStyle::Atx,
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
        HtmlMdLinkStyle::CollapsedReference => o.link_reference_style = LinkReferenceStyle::Collapsed,
        HtmlMdLinkStyle::ShortcutReference => o.link_reference_style = LinkReferenceStyle::Shortcut,
        _ => {}
    }

    // GFM / Extended profiles enable tables and task lists by keeping the
    // `htmd` default behavior (it always converts tables to GFM pipe tables).
    o.translation_mode = match options.render.raw_html_policy {
        RawHtmlPolicy::Faithful => TranslationMode::Faithful,
        _ => TranslationMode::Pure,
    };

    o
}

fn post_process(markdown: &str, options: &ConversionOptions) -> String {
    let mut s = markdown.to_string();

    if options.render.trailing_whitespace == WhitespacePolicy::Trim {
        s = s
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
    }

    match options.render.final_newline {
        FinalNewlinePolicy::Ensure => {
            if !s.ends_with('\n') {
                s.push('\n');
            }
        }
        FinalNewlinePolicy::Suppress => {
            s = s.trim_end_matches('\n').to_string();
        }
        FinalNewlinePolicy::Preserve => {}
    }

    if options.render.unicode_normalization == crate::options::UnicodeNormalization::Nfc {
        use unicode_normalization::UnicodeNormalization;
        s = s.nfc().collect();
    } else if options.render.unicode_normalization == crate::options::UnicodeNormalization::Nfkc {
        use unicode_normalization::UnicodeNormalization;
        s = s.nfkc().collect();
    }

    s
}

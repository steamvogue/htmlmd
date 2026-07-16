// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::backend::ConverterBackend;
use crate::error::Result;
use crate::htmd_handlers;
use crate::options::{ConversionOptions, FinalNewlinePolicy, WhitespacePolicy};
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
        let markdown = htmd_handlers::convert_with_htmd(html, options)?;
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

/// Shared render post-processing (trailing whitespace, final newline, Unicode
/// normalization). Also used by `NativeBackend`, whose output must stay
/// byte-identical to this backend's.
pub(crate) fn post_process(markdown: &str, options: &ConversionOptions) -> String {
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

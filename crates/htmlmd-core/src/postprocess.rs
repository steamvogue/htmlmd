// SPDX-License-Identifier: MIT OR Apache-2.0

//! Render post-processing shared by every backend. Output must stay
//! byte-identical across backends, so all of them funnel through here.

use crate::options::{ConversionOptions, FinalNewlinePolicy, WhitespacePolicy};

/// Apply trailing-whitespace, final-newline, and Unicode-normalization
/// policies to rendered Markdown.
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

    if options.render.normalize_whitespace {
        s = s.replace(['\u{00A0}', '\u{2007}', '\u{202F}'], " ");
    }

    s
}

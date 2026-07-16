// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::{Error, Result};
use crate::options::ConversionOptions;

/// Validate a `ConversionOptions` value and return a configuration error on the
/// first problem. This is used by the CLI before any files are processed.
pub fn validate(options: &ConversionOptions) -> Result<()> {
    validate_selectors(options)?;
    validate_base_url(options)?;
    validate_regexes(options)?;
    Ok(())
}

fn validate_selectors(options: &ConversionOptions) -> Result<()> {
    let all: Vec<(String, &str)> = [
        (
            "cleanup.remove-selectors",
            &options.cleanup.remove_selectors,
        ),
        (
            "cleanup.unwrap-selectors",
            &options.cleanup.unwrap_selectors,
        ),
        (
            "cleanup.keep-only-selectors",
            &options.cleanup.keep_only_selectors,
        ),
    ]
    .into_iter()
    .flat_map(|(key, list)| list.iter().map(move |s| (key.to_string(), s.as_str())))
    .chain(
        options
            .cleanup
            .extract_selector
            .iter()
            .map(|s| ("cleanup.extract-selector".to_string(), s.as_str())),
    )
    .chain(
        options
            .cleanup
            .main_content_selector
            .iter()
            .map(|s| ("cleanup.main-content-selector".to_string(), s.as_str())),
    )
    .collect();

    for (key, selector) in all {
        if let Err(e) = scraper::Selector::parse(selector) {
            return Err(Error::Config(format!(
                "invalid selector in {key} ('{selector}'): {e}"
            )));
        }
    }
    Ok(())
}

fn validate_base_url(options: &ConversionOptions) -> Result<()> {
    if let Some(base) = &options.cleanup.base_url {
        url::Url::parse(base)
            .map_err(|e| Error::Config(format!("invalid base-url '{base}': {e}")))?;
    }
    Ok(())
}

fn validate_regexes(options: &ConversionOptions) -> Result<()> {
    for (i, rule) in options.cleanup.url_rewrite_rules.iter().enumerate() {
        regex::Regex::new(&rule.pattern).map_err(|e| {
            Error::Config(format!(
                "invalid regex in url-rewrite-rules[{i}] ('{}'): {e}",
                rule.pattern
            ))
        })?;
    }
    for (i, pat) in options.semantic.code_language_patterns.iter().enumerate() {
        regex::Regex::new(pat).map_err(|e| {
            Error::Config(format!(
                "invalid regex in code-language-patterns[{i}] ('{pat}'): {e}"
            ))
        })?;
    }
    Ok(())
}

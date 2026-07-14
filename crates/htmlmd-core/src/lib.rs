// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod backend;
pub mod cleanup;
pub mod diagnostic;
pub mod error;
pub mod htmd_backend;
mod htmd_handlers;
pub mod options;
pub mod result;
pub mod rewrite;

use std::io::Write;

pub use backend::ConverterBackend;
pub use error::{Error, Result};
pub use htmd_backend::HtmdBackend;
pub use options::ConversionOptions;
pub use result::ConversionResult;

pub use cleanup::{clean_html, ExtractedMetadata};
use diagnostic::Diagnostic;

/// Convert a UTF-8 HTML string to Markdown using the default backend.
///
/// # Example
///
/// ```
/// use htmlmd_core::{convert, ConversionOptions};
///
/// let md = convert("<h1>Hello</h1>", &ConversionOptions::default()).unwrap();
/// assert_eq!(md.markdown.trim(), "# Hello");
/// ```
pub fn convert(html: &str, options: &ConversionOptions) -> Result<ConversionResult> {
    let backend = HtmdBackend::new();
    convert_with_backend(html, options, &backend)
}

/// Convert with a specific backend implementation.
pub fn convert_with_backend<B: ConverterBackend + ?Sized>(
    html: &str,
    options: &ConversionOptions,
    backend: &B,
) -> Result<ConversionResult> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    if options.limits.max_input_bytes > 0 && html.len() as u64 > options.limits.max_input_bytes {
        return Err(Error::LimitExceeded(format!(
            "input size {} exceeds {}",
            html.len(),
            options.limits.max_input_bytes
        )));
    }

    let (cleaned_html, metadata) = clean_html(html, options, None, &mut diagnostics)?;

    let mut result = backend.convert(&cleaned_html, options)?;

    result.title = metadata.title.or(result.title);
    result.description = metadata.description.or(result.description);
    result.canonical_url = metadata.canonical_url.or(result.canonical_url);
    result.diagnostics.extend(diagnostics);

    apply_profile_post_processing(&mut result, options);

    if options.strict && result.has_errors() {
        return Err(Error::Other("conversion produced errors in strict mode".to_string()));
    }

    Ok(result)
}

/// Convert and stream the Markdown output directly to a writer.
pub fn convert_to_writer(
    html: &str,
    options: &ConversionOptions,
    writer: &mut dyn Write,
) -> Result<()> {
    let result = convert(html, options)?;
    writer.write_all(result.markdown.as_bytes())?;
    Ok(())
}

/// Convert with a backend and stream output to a writer.
pub fn convert_with_backend_to_writer<B: ConverterBackend + ?Sized>(
    html: &str,
    options: &ConversionOptions,
    backend: &B,
    writer: &mut dyn Write,
) -> Result<()> {
    let result = convert_with_backend(html, options, backend)?;
    writer.write_all(result.markdown.as_bytes())?;
    Ok(())
}


fn apply_profile_post_processing(result: &mut ConversionResult, options: &ConversionOptions) {
    match options.profile {
        crate::options::OutputProfile::Obsidian => {
            if let Some(frontmatter) = build_obsidian_frontmatter(result) {
                result.markdown = format!("{frontmatter}\n{}", result.markdown);
            }
        }
        crate::options::OutputProfile::PlainText => {
            result.markdown = strip_markdown(&result.markdown);
        }
        crate::options::OutputProfile::MdxSafe => {
            result.markdown = escape_mdx(&result.markdown);
        }
        _ => {}
    }
}

fn build_obsidian_frontmatter(result: &ConversionResult) -> Option<String> {
    let mut lines = Vec::new();
    if let Some(title) = &result.title {
        lines.push(format!("title: {title}"));
    }
    if let Some(description) = &result.description {
        lines.push(format!("description: {description}"));
    }
    if let Some(url) = &result.canonical_url {
        lines.push(format!("canonical_url: {url}"));
    }
    if lines.is_empty() {
        return None;
    }
    Some(format!("---\n{}\n---", lines.join("\n")))
}

fn escape_mdx(text: &str) -> String {
    // Escape curly braces, which MDX interprets as JSX expressions.
    text.replace('{', "\\{").replace('}', "\\}")
}

fn strip_markdown(text: &str) -> String {
    use regex::Regex;

    let mut s = text.to_string();

    // Headings: remove ATX markers.
    s = Regex::new(r"(?m)^#{1,6}\s+").unwrap().replace_all(&s, "").to_string();
    // Blockquote markers.
    s = Regex::new(r"(?m)^>\s?").unwrap().replace_all(&s, "").to_string();
    // List bullets / ordered markers.
    s = Regex::new(r"(?m)^[-*+]\s+").unwrap().replace_all(&s, "").to_string();
    s = Regex::new(r"(?m)^\d+\.\s+").unwrap().replace_all(&s, "").to_string();
    // Fenced code blocks.
    s = Regex::new(r"```[\s\S]*?```").unwrap().replace_all(&s, "").to_string();
    s = Regex::new(r"~~~[\s\S]*?~~~").unwrap().replace_all(&s, "").to_string();
    // Inline code.
    s = Regex::new(r"`([^`]+)`").unwrap().replace_all(&s, "$1").to_string();
    // Images -> alt text.
    s = Regex::new(r"!\[([^\]]*)\]\([^)]*\)").unwrap().replace_all(&s, "$1").to_string();
    // Links -> link text.
    s = Regex::new(r"\[([^\]]+)\]\([^)]*\)").unwrap().replace_all(&s, "$1").to_string();
    // Emphasis / highlight / insert / strike / sub / sup markers.
    for marker in ["**", "__", "*", "_", "~~", "==", "++", "^", "~"] {
        s = s.replace(marker, "");
    }
    // Horizontal rules.
    s = Regex::new(r"(?m)^---+\s*$").unwrap().replace_all(&s, "").to_string();

    // Collapse multiple blank lines.
    Regex::new(r"\n{3,}").unwrap().replace_all(s.trim(), "\n\n").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_heading() {
        let md = convert("<h1>Hello World</h1>", &ConversionOptions::default()).unwrap();
        assert_eq!(md.markdown.trim(), "# Hello World");
    }

    #[test]
    fn gfm_table() {
        let html = "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>";
        let md = convert(html, &ConversionOptions::gfm()).unwrap();
        assert!(md.markdown.contains("| A | B |"));
        assert!(md.markdown.contains("| 1 | 2 |"));
    }

    #[test]
    fn remove_script_tag() {
        let md = convert(
            "<p>hello</p><script>alert(1)</script>",
            &ConversionOptions::default(),
        )
        .unwrap();
        assert!(!md.markdown.contains("script"));
        assert!(md.markdown.contains("hello"));
    }
}

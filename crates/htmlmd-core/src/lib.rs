// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod backend;
pub mod cleanup;
#[doc(hidden)]
pub mod corpus;
pub mod diagnostic;
pub mod error;
#[cfg(feature = "backend-htmd")]
pub mod htmd_backend;
#[cfg(feature = "backend-htmd")]
mod htmd_handlers;
pub(crate) mod native;
pub mod options;
mod postprocess;
mod regex_cache;
pub mod result;
pub mod rewrite;

use std::io::Write;

pub use backend::ConverterBackend;
pub use error::{Error, Result};
#[cfg(feature = "backend-htmd")]
pub use htmd_backend::HtmdBackend;
pub use native::NativeBackend;
pub use options::ConversionOptions;
pub use result::ConversionResult;

pub use cleanup::{ExtractedMetadata, clean_html, clean_html_to_dom};
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
    let backend = NativeBackend::new();
    convert_with_backend(html, options, &backend)
}

/// Convert with a specific backend implementation.
pub fn convert_with_backend<B: ConverterBackend + ?Sized>(
    html: &str,
    options: &ConversionOptions,
    backend: &B,
) -> Result<ConversionResult> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // All input limits (including max_input_bytes) are enforced in one place:
    // `cleanup::check_limits`, which errors in strict mode and warns otherwise.
    let (document, metadata) = clean_html_to_dom(html, options, None, &mut diagnostics)?;

    // Backends that don't override `convert_dom` get the serialize-and-convert
    // default, which is byte-identical to the previous
    // `clean_html` + `convert(&cleaned_html)` string path. `NativeBackend`
    // overrides it to render straight from the cleaned DOM (single parse).
    let mut result = backend.convert_dom(&document, options)?;

    result.title = metadata.title.or(result.title);
    result.description = metadata.description.or(result.description);
    result.canonical_url = metadata.canonical_url.or(result.canonical_url);
    result.diagnostics.extend(diagnostics);

    apply_profile_post_processing(&mut result, options);

    if options.limits.max_output_bytes > 0
        && result.markdown.len() as u64 > options.limits.max_output_bytes
    {
        let msg = format!(
            "output size {} exceeds limit {}",
            result.markdown.len(),
            options.limits.max_output_bytes
        );
        if options.strict {
            return Err(Error::LimitExceeded(msg));
        }
        result.diagnostics.push(Diagnostic::warning(msg));
    }

    if options.strict && result.has_errors() {
        return Err(Error::Other(
            "conversion produced errors in strict mode".to_string(),
        ));
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
        lines.push(format!("title: {}", yaml_quote(title)));
    }
    if let Some(description) = &result.description {
        lines.push(format!("description: {}", yaml_quote(description)));
    }
    if let Some(url) = &result.canonical_url {
        lines.push(format!("canonical_url: {}", yaml_quote(url)));
    }
    if lines.is_empty() {
        return None;
    }
    Some(format!("---\n{}\n---", lines.join("\n")))
}

/// Quote a string as a double-quoted YAML scalar so metadata containing
/// `:`, `#`, quotes, or newlines cannot break the frontmatter.
fn yaml_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn escape_mdx(text: &str) -> String {
    // Escape curly braces (JSX expressions) and `<` where it could open a
    // JSX element or fragment. A bare `<` followed by whitespace or a digit
    // is left alone — MDX treats it as text.
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '<' => {
                if matches!(chars.peek(), Some(n) if n.is_ascii_alphabetic() || *n == '/' || *n == '>')
                {
                    out.push_str("\\<");
                } else {
                    out.push('<');
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn strip_markdown(text: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;

    // htmd escapes literal Markdown metacharacters in prose (`snake\_case`,
    // `a\*b`). Encode each backslash-escaped character into the Unicode
    // private-use area so the structural-marker stripping below cannot see
    // it, then map it back to the bare character at the end.
    const PUA_BASE: u32 = 0xE100;
    fn protect(c: char) -> char {
        char::from_u32(PUA_BASE + c as u32).unwrap_or(c)
    }
    fn unprotect(c: char) -> char {
        let cp = c as u32;
        if (PUA_BASE..PUA_BASE + 0x80).contains(&cp) {
            char::from_u32(cp - PUA_BASE).unwrap_or(c)
        } else {
            c
        }
    }

    static HEADINGS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^#{1,6}\s+").unwrap());
    static BLOCKQUOTES: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^>\s?").unwrap());
    static BULLETS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^[-*+]\s+").unwrap());
    static ORDERED: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\d+\.\s+").unwrap());
    // Fenced blocks: drop the fence lines, keep the code itself readable.
    static FENCED_BACKTICK: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"```[^\n]*\n?([\s\S]*?)```").unwrap());
    static FENCED_TILDE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"~~~[^\n]*\n?([\s\S]*?)~~~").unwrap());
    static INLINE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`([^`]+)`").unwrap());
    static IMAGES: Lazy<Regex> = Lazy::new(|| Regex::new(r"!\[([^\]]*)\]\([^)]*\)").unwrap());
    static LINKS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]+)\]\([^)]*\)").unwrap());
    // Emphasis-family markers are stripped only in *pairs*, so unpaired
    // characters in prose or code (`x^2`, `C++`, `b * c`) survive.
    static BOLD_STARS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*\*([^*]+)\*\*").unwrap());
    static BOLD_UNDERS: Lazy<Regex> = Lazy::new(|| Regex::new(r"__([^_]+)__").unwrap());
    static EM_STAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*([^*\n]+)\*").unwrap());
    static EM_UNDER: Lazy<Regex> = Lazy::new(|| Regex::new(r"_([^_\n]+)_").unwrap());
    static STRIKE: Lazy<Regex> = Lazy::new(|| Regex::new(r"~~([^~]+)~~").unwrap());
    static HIGHLIGHT: Lazy<Regex> = Lazy::new(|| Regex::new(r"==([^=\n]+)==").unwrap());
    static INSERT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\+\+([^+\n]+)\+\+").unwrap());
    static SUPERSCRIPT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\^([^^\s]+)\^").unwrap());
    static SUBSCRIPT: Lazy<Regex> = Lazy::new(|| Regex::new(r"~([^~\s]+)~").unwrap());
    static HRULES: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^---+\s*$").unwrap());
    static BLANK_LINES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());

    // 1. Protect backslash escapes.
    let mut s = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_punctuation() {
                    s.push(protect(next));
                    chars.next();
                    continue;
                }
            }
        }
        s.push(c);
    }

    // 2. Code: unwrap fenced blocks and inline code, keeping content.
    s = FENCED_BACKTICK.replace_all(&s, "$1").to_string();
    s = FENCED_TILDE.replace_all(&s, "$1").to_string();
    s = INLINE_CODE.replace_all(&s, "$1").to_string();

    // 3. Images -> alt text, links -> link text.
    s = IMAGES.replace_all(&s, "$1").to_string();
    s = LINKS.replace_all(&s, "$1").to_string();

    // 4. Paired inline markers (longest first).
    s = BOLD_STARS.replace_all(&s, "$1").to_string();
    s = BOLD_UNDERS.replace_all(&s, "$1").to_string();
    s = EM_STAR.replace_all(&s, "$1").to_string();
    s = EM_UNDER.replace_all(&s, "$1").to_string();
    s = STRIKE.replace_all(&s, "$1").to_string();
    s = HIGHLIGHT.replace_all(&s, "$1").to_string();
    s = INSERT.replace_all(&s, "$1").to_string();
    s = SUPERSCRIPT.replace_all(&s, "$1").to_string();
    s = SUBSCRIPT.replace_all(&s, "$1").to_string();

    // 5. Block markers.
    s = HEADINGS.replace_all(&s, "").to_string();
    s = BLOCKQUOTES.replace_all(&s, "").to_string();
    s = BULLETS.replace_all(&s, "").to_string();
    s = ORDERED.replace_all(&s, "").to_string();
    s = HRULES.replace_all(&s, "").to_string();

    // 6. Restore protected literals (dropping the escape backslash).
    s = s.chars().map(unprotect).collect();

    // Collapse multiple blank lines.
    BLANK_LINES.replace_all(s.trim(), "\n\n").to_string()
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
    fn plain_text_keeps_literal_metacharacters() {
        let html = "<p>snake_case a*b x^2 C++ 2_10</p>";
        let md = convert(html, &ConversionOptions::plain_text()).unwrap();
        assert_eq!(md.markdown.trim(), "snake_case a*b x^2 C++ 2_10");
    }

    #[test]
    fn plain_text_keeps_code_content() {
        let html = "<pre><code>let a = b * c;</code></pre><p>uses <code>x_y</code></p>";
        let md = convert(html, &ConversionOptions::plain_text()).unwrap();
        assert!(md.markdown.contains("let a = b * c;"), "{}", md.markdown);
        assert!(md.markdown.contains("uses x_y"), "{}", md.markdown);
    }

    #[test]
    fn plain_text_strips_paired_markers() {
        let html = "<p><strong>bold</strong> and <em>em</em> and <mark>hi</mark></p>";
        let md = convert(html, &ConversionOptions::plain_text()).unwrap();
        assert_eq!(md.markdown.trim(), "bold and em and hi");
    }

    #[test]
    fn obsidian_frontmatter_is_yaml_safe() {
        let html = "<html><head><title>Rust: a story #1</title></head>\
                    <body><p>hi</p></body></html>";
        let mut options = ConversionOptions::obsidian();
        options.cleanup.metadata.title = true;
        let md = convert(html, &options).unwrap();
        assert!(
            md.markdown
                .starts_with("---\ntitle: \"Rust: a story #1\"\n---"),
            "{}",
            md.markdown
        );
    }

    #[test]
    fn mdx_escapes_jsx_openers() {
        let html = "<p>use x = 1 and a &lt; b</p>";
        let md = convert(html, &ConversionOptions::mdx_safe()).unwrap();
        assert!(md.markdown.contains("a < b"), "{}", md.markdown);
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

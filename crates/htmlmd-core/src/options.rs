// SPDX-License-Identifier: MIT OR Apache-2.0

//! Configuration options for HTML to Markdown conversion.
//!
//! `ConversionOptions` is the central configuration type. It is serializable with
//! `serde`, so the same structure can be populated from Rust, JSON, TOML, or
//! environment variables (via the CLI's configuration loader).
//!
//! # Phase 1 scope note
//!
//! The schema intentionally reserves fields for Phase 2+ features (e.g. math,
//! definition lists, custom rules) so the public API stays stable. Options that
//! are wired to the Phase 1 backend are marked in their doc comments; the rest
//! are accepted, parsed, and documented, but do not yet affect output.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub mod validation;

/// Output profile. Determines the default set of Markdown extensions and rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputProfile {
    /// Conservative portable CommonMark.
    #[default]
    Commonmark,
    /// GitHub Flavored Markdown (tables, strikethrough, task lists, autolinks).
    Gfm,
    /// GFM plus footnotes, definition lists, alerts, code-block attributes, math.
    Extended,
    /// Pandoc-friendly extensions and attributes.
    Pandoc,
    /// Obsidian-style wikilinks, callouts, frontmatter.
    Obsidian,
    /// Escape or retain constructs that could be interpreted as JSX/MDX.
    MdxSafe,
    /// Readable plain text output.
    PlainText,
}

/// Heading marker style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeadingStyle {
    /// ATX style: `# Heading`.
    #[default]
    Atx,
    /// Setext style: underlined H1/H2.
    Setex,
    /// Keep the original HTML heading representation when no Markdown equivalent exists.
    Keep,
}

/// Bullet list marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BulletMarker {
    /// `- item`
    #[default]
    Hyphen,
    /// `* item`
    Asterisk,
    /// `+ item`
    Plus,
}

/// Ordered list numbering strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderedListMarker {
    /// `1. 2. 3.`
    #[default]
    Decimal,
    /// `01.`
    ZeroPadded,
    /// Roman numerals.
    Roman,
    /// `a.`/`A.`
    Alpha,
    /// `1)`
    OneDot,
}

/// Inline emphasis marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmphasisMarker {
    /// `*emphasis*`
    #[default]
    Asterisk,
    /// `_emphasis_`
    Underscore,
}

/// Strong emphasis marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StrongMarker {
    /// `**strong**`
    #[default]
    Asterisk,
    /// `__strong__`
    Underscore,
}

/// Code fence delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodeFence {
    /// ` ```code``` `
    #[default]
    Backticks,
    /// `~~~code~~~`
    Tildes,
}

/// Line wrapping policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LineWrapping {
    /// No wrapping.
    #[default]
    Off,
    /// Hard wrap at a fixed column.
    Fixed(u32),
    /// Wrap at word boundaries near the column.
    Semantic(u32),
}

/// Hard line break representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardBreakStyle {
    /// Two trailing spaces.
    #[default]
    TwoSpaces,
    /// Backslash.
    Backslash,
}

/// Horizontal rule style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HrStyle {
    /// `---`
    #[default]
    Dashes,
    /// `***`
    Asterisks,
    /// `___`
    Underscores,
}

/// Markdown escaping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EscapingMode {
    /// Escape only when required by the target profile.
    #[default]
    Minimal,
    /// Escape common ambiguous characters.
    Conservative,
    /// Escape aggressively for maximum safety.
    Strict,
}

/// HTML character entity policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntityPolicy {
    /// Decode entities to Unicode.
    #[default]
    Decode,
    /// Preserve the original entity references.
    Preserve,
}

/// Unicode normalization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnicodeNormalization {
    /// No normalization.
    #[default]
    Off,
    /// NFC.
    Nfc,
    /// NFKC.
    Nfkc,
}

/// Smart punctuation handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SmartPunctuation {
    /// Keep as-is.
    #[default]
    Preserve,
    /// Normalize to curly quotes and em-dashes.
    Normalize,
    /// Convert to ASCII equivalents.
    Ascii,
}

/// Trailing whitespace policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WhitespacePolicy {
    /// Trim trailing whitespace.
    #[default]
    Trim,
    /// Preserve trailing whitespace.
    Preserve,
}

/// Final newline policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinalNewlinePolicy {
    /// Ensure a single trailing newline.
    #[default]
    Ensure,
    /// Preserve the original final newline state.
    Preserve,
    /// Suppress the final newline.
    Suppress,
}

/// Link/reference style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkStyle {
    /// `[text](url)`
    #[default]
    Inline,
    /// `[text][id]` with definitions at the configured placement.
    Reference,
    /// `[text][]`
    CollapsedReference,
    /// `[text]`
    ShortcutReference,
}

/// Placement of reference link definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReferencePlacement {
    /// At the end of the document.
    #[default]
    End,
    /// At the end of the nearest section.
    SectionEnd,
    /// Immediately after the link.
    Adjacent,
}

/// How images are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageMode {
    /// `![alt](url)`
    #[default]
    Inline,
    /// Reference-style images.
    Reference,
    /// Drop images entirely.
    Skip,
    /// Render only alt text.
    AltText,
}

/// How `title` attributes are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TitleHandling {
    /// Ignore titles.
    #[default]
    Ignore,
    /// Include titles inline.
    Inline,
    /// Move titles to reference definitions.
    Reference,
}

/// URL escaping / sanitization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UrlEscaping {
    /// Escape only when necessary.
    #[default]
    Auto,
    /// Never escape URLs.
    Never,
    /// Always percent-encode.
    Always,
}

/// Email address handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmailHandling {
    /// Convert to `mailto:` links.
    #[default]
    Mailto,
    /// Render as plain text.
    Plain,
    /// Obfuscate.
    Obfuscate,
}

/// What to do with unsupported raw HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RawHtmlPolicy {
    /// Drop unsupported HTML.
    #[default]
    Drop,
    /// Preserve raw HTML in the output.
    Preserve,
    /// Escape HTML to entities.
    Escape,
    /// Embed HTML faithfully when round-tripping matters.
    Faithful,
}

/// HTML comment handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommentPolicy {
    /// Drop comments.
    #[default]
    Drop,
    /// Preserve raw `<!-- -->`.
    Preserve,
    /// Convert to Markdown comments (`<!-- -->` is the same in Markdown).
    Markdown,
}

/// DOCTYPE / XML processing instruction handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DoctypePolicy {
    /// Drop declarations.
    #[default]
    Drop,
    /// Preserve them.
    Preserve,
}

/// Hidden content policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HiddenContentPolicy {
    /// Include hidden content.
    #[default]
    Show,
    /// Remove content that is CSS hidden, aria-hidden, or has the hidden attribute.
    Hide,
}

/// Responsive image selection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResponsiveImagePolicy {
    /// Pick the first `srcset` candidate.
    #[default]
    FirstSrcset,
    /// Pick the largest descriptor.
    Largest,
    /// Leave the `srcset` attribute as-is.
    PreserveSrcset,
}

/// Generic media handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MediaPolicy {
    /// Convert to inline Markdown where possible.
    #[default]
    Inline,
    /// Convert to a plain link.
    Link,
    /// Replace with a placeholder.
    Placeholder,
    /// Drop the element.
    Drop,
}

/// Form handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormHandling {
    /// Drop the form.
    #[default]
    Drop,
    /// Render readable text labels and values.
    Readable,
    /// Render as checklist.
    Checklist,
}

/// `<details>` / `<summary>` handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetailsHandling {
    /// Expand details into body text.
    #[default]
    Expand,
    /// Keep only the summary line.
    SummaryOnly,
    /// Drop the entire element.
    Drop,
}

/// Policy for custom / unknown elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CustomElementPolicy {
    /// Unwrap children.
    #[default]
    Unwrap,
    /// Drop the element.
    Drop,
    /// Preserve as raw HTML.
    PreserveHtml,
}

/// Table conversion strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TableHandling {
    /// GFM pipe tables.
    #[default]
    Gfm,
    /// Fall back to raw HTML.
    HtmlFallback,
    /// Flatten cells into paragraphs.
    Flatten,
    /// CSV-like text.
    CsvLike,
    /// Drop tables.
    Drop,
}

/// Strategy for rowspan / colspan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DifficultTableStrategy {
    /// Use raw HTML for complex tables.
    #[default]
    HtmlFallback,
    /// Replicate span cells.
    SpanCells,
    /// Flatten the table.
    Flatten,
}

/// CSS-aware inline style conversion subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InlineStyleSubset {
    /// Do not consider inline styles.
    #[default]
    Off,
    /// Convert a documented safe subset.
    Basic,
    /// Convert all recognized styles.
    All,
}

/// Semantic tag handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticTagPolicy {
    /// Convert to Markdown equivalents.
    #[default]
    Convert,
    /// Preserve as raw HTML.
    PreserveHtml,
    /// Drop the element.
    Drop,
}

/// Math output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MathOutput {
    /// Preserve original HTML/math markup.
    #[default]
    PreserveHtml,
    /// `$...$`
    InlineDollar,
    /// `$$...$$`
    BlockDollar,
    /// Fenced math block.
    Fenced,
    /// Plain text.
    Plain,
}

/// Mermaid / diagram handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MermaidPolicy {
    /// Output fenced code blocks with language identifier.
    #[default]
    Fenced,
    /// Preserve HTML.
    PreserveHtml,
    /// Drop.
    Drop,
}

/// Embedded social/media widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddedMediaPolicy {
    /// Preserve the link if one exists.
    #[default]
    PreserveLink,
    /// Drop the widget.
    Drop,
    /// Render a placeholder.
    Placeholder,
}

/// Action for a custom tag rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CustomRuleAction {
    /// Drop the element.
    #[default]
    Drop,
    /// Unwrap children.
    Unwrap,
    /// Render children as plain text.
    Text,
    /// Preserve raw HTML.
    Html,
    /// Render a Markdown template.
    MarkdownTemplate,
    /// Render as a fenced block.
    FencedBlock,
    /// Render as a link.
    Link,
    /// Render as an image.
    Image,
}

/// Metadata extraction options.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MetadataOptions {
    pub title: bool,
    pub description: bool,
    pub canonical_url: bool,
    pub open_graph_title: bool,
    pub open_graph_description: bool,
    pub open_graph_image: bool,
    pub twitter_title: bool,
    pub twitter_description: bool,
}

/// A single URL rewrite rule.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UrlRewriteRule {
    pub pattern: String,
    pub replacement: String,
}

/// Safety and size limits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Limits {
    /// Maximum input size in bytes. Zero means unlimited.
    pub max_input_bytes: u64,
    /// Maximum output size in bytes. Zero means unlimited.
    pub max_output_bytes: u64,
    /// Maximum DOM depth. Zero means unlimited.
    pub max_dom_depth: u32,
    /// Maximum DOM node count. Zero means unlimited.
    pub max_node_count: u64,
    /// Maximum length of a single attribute value. Zero means unlimited.
    pub max_attribute_len: u64,
}

/// Math detection and output options.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MathOptions {
    pub enabled: bool,
    pub output: MathOutput,
    pub mathjax_selectors: Vec<String>,
    pub katex_selectors: Vec<String>,
    pub mathml_selectors: Vec<String>,
    pub custom_selectors: Vec<String>,
}

/// A custom conversion rule.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CustomRule {
    pub selectors: Vec<String>,
    pub action: CustomRuleAction,
    pub template: Option<String>,
    pub priority: i32,
}

/// Markdown rendering options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RenderOptions {
    pub heading_style: HeadingStyle,
    pub bullet_marker: BulletMarker,
    pub ordered_list_marker: OrderedListMarker,
    pub emphasis_marker: EmphasisMarker,
    pub strong_marker: StrongMarker,
    pub code_fence: CodeFence,
    /// Minimum number of backticks/tildes in a code fence. Reserved for Phase 2+.
    pub code_fence_min_length: u8,
    pub line_wrapping: LineWrapping,
    pub hard_break_style: HardBreakStyle,
    pub hr_style: HrStyle,
    pub escaping_mode: EscapingMode,
    pub character_entities: EntityPolicy,
    pub unicode_normalization: UnicodeNormalization,
    pub smart_punctuation: SmartPunctuation,
    pub trailing_whitespace: WhitespacePolicy,
    pub final_newline: FinalNewlinePolicy,
    /// 0 = no compaction, 1 = collapse 2+ blank lines, 2 = collapse all extra blanks.
    pub blank_line_compaction: u8,
    pub link_style: LinkStyle,
    pub reference_placement: ReferencePlacement,
    pub image_mode: ImageMode,
    pub title_attribute: TitleHandling,
    pub url_escaping: UrlEscaping,
    pub autolink_detection: bool,
    pub email_handling: EmailHandling,
    pub raw_html_policy: RawHtmlPolicy,
    pub comment_policy: CommentPolicy,
    pub doctype_policy: DoctypePolicy,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            heading_style: HeadingStyle::Atx,
            bullet_marker: BulletMarker::Hyphen,
            ordered_list_marker: OrderedListMarker::Decimal,
            emphasis_marker: EmphasisMarker::Asterisk,
            strong_marker: StrongMarker::Asterisk,
            code_fence: CodeFence::Backticks,
            code_fence_min_length: 3,
            line_wrapping: LineWrapping::Off,
            hard_break_style: HardBreakStyle::TwoSpaces,
            hr_style: HrStyle::Dashes,
            escaping_mode: EscapingMode::Minimal,
            character_entities: EntityPolicy::Decode,
            unicode_normalization: UnicodeNormalization::Off,
            smart_punctuation: SmartPunctuation::Preserve,
            trailing_whitespace: WhitespacePolicy::Trim,
            final_newline: FinalNewlinePolicy::Ensure,
            blank_line_compaction: 1,
            link_style: LinkStyle::Inline,
            reference_placement: ReferencePlacement::End,
            image_mode: ImageMode::Inline,
            title_attribute: TitleHandling::Ignore,
            url_escaping: UrlEscaping::Auto,
            autolink_detection: true,
            email_handling: EmailHandling::Mailto,
            raw_html_policy: RawHtmlPolicy::Drop,
            comment_policy: CommentPolicy::Drop,
            doctype_policy: DoctypePolicy::Drop,
        }
    }
}

/// HTML cleanup and content selection options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CleanupOptions {
    /// CSS selectors whose matched elements are removed.
    pub remove_selectors: Vec<String>,
    /// CSS selectors whose matched elements are unwrapped (children kept).
    pub unwrap_selectors: Vec<String>,
    /// If non-empty, only content inside these selectors is retained.
    pub keep_only_selectors: Vec<String>,
    /// Extract only this selector (alias for keep-only with fallback to body).
    pub extract_selector: Option<String>,
    /// Selector used for main-content extraction.
    pub main_content_selector: Option<String>,
    /// Tag names that are always removed (e.g. script, style).
    pub remove_tags: Vec<String>,
    /// Per-tag behavior overrides. Map of tag name -> action.
    pub per_tag_behavior: indexmap::IndexMap<String, String>,
    pub hidden_content_policy: HiddenContentPolicy,
    pub metadata: MetadataOptions,
    /// Base URL for resolving relative links and images.
    pub base_url: Option<String>,
    pub url_rewrite_rules: Vec<UrlRewriteRule>,
    /// Remove common tracking parameters.
    pub remove_tracking_params: bool,
    /// Additional tracking parameter names to strip.
    pub tracking_params: Vec<String>,
    /// URL schemes explicitly allowed.
    pub allowed_url_schemes: Vec<String>,
    /// URL schemes explicitly blocked.
    pub blocked_url_schemes: Vec<String>,
    /// Attributes to consider for lazy-loaded images.
    pub lazy_image_attrs: Vec<String>,
    pub responsive_image_policy: ResponsiveImagePolicy,
    /// Preserve width/height attributes in image alt/title metadata.
    pub preserve_image_metadata: bool,
    pub image_mode: ImageMode,
    pub media_policy: MediaPolicy,
    pub form_handling: FormHandling,
    pub details_handling: DetailsHandling,
    pub custom_element_policy: CustomElementPolicy,
}

impl Default for CleanupOptions {
    fn default() -> Self {
        Self {
            remove_selectors: Vec::new(),
            unwrap_selectors: Vec::new(),
            keep_only_selectors: Vec::new(),
            extract_selector: None,
            main_content_selector: None,
            remove_tags: vec![
                "head".to_string(),
                "script".to_string(),
                "style".to_string(),
                "noscript".to_string(),
                "template".to_string(),
                "iframe".to_string(),
                "svg".to_string(),
                "canvas".to_string(),
            ],
            per_tag_behavior: IndexMap::new(),
            hidden_content_policy: HiddenContentPolicy::Hide,
            metadata: MetadataOptions::default(),
            base_url: None,
            url_rewrite_rules: Vec::new(),
            remove_tracking_params: true,
            tracking_params: Vec::new(),
            allowed_url_schemes: Vec::new(),
            blocked_url_schemes: vec![
                "javascript".to_string(),
                "data".to_string(),
                "file".to_string(),
            ],
            lazy_image_attrs: vec!["data-src".to_string(), "data-original".to_string()],
            responsive_image_policy: ResponsiveImagePolicy::FirstSrcset,
            preserve_image_metadata: false,
            image_mode: ImageMode::Inline,
            media_policy: MediaPolicy::Inline,
            form_handling: FormHandling::Drop,
            details_handling: DetailsHandling::Expand,
            custom_element_policy: CustomElementPolicy::Unwrap,
        }
    }
}

/// Semantic conversion options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SemanticOptions {
    pub heading_offset: i8,
    pub normalize_headings: bool,
    pub list_indent: u8,
    pub task_lists: bool,
    pub table_handling: TableHandling,
    pub difficult_table_strategy: DifficultTableStrategy,
    pub code_language_patterns: Vec<String>,
    /// Try to infer a code-block language from the source text when no
    /// `language-*` class is present.
    pub detect_languages: bool,
    pub inline_style_subset: InlineStyleSubset,
    pub semantic_tags: SemanticTagPolicy,
    pub definition_lists: bool,
    pub footnotes: bool,
    pub math: MathOptions,
    pub mermaid: MermaidPolicy,
    pub embedded_media: EmbeddedMediaPolicy,
}

impl Default for SemanticOptions {
    fn default() -> Self {
        Self {
            heading_offset: 0,
            normalize_headings: false,
            list_indent: 4,
            task_lists: true,
            table_handling: TableHandling::Gfm,
            difficult_table_strategy: DifficultTableStrategy::HtmlFallback,
            code_language_patterns: vec![
                r"language-(?P<lang>\S+)".to_string(),
                r"lang-(?P<lang>\S+)".to_string(),
                r"highlight-(?P<lang>\S+)".to_string(),
            ],
            detect_languages: true,
            inline_style_subset: InlineStyleSubset::Basic,
            semantic_tags: SemanticTagPolicy::Convert,
            definition_lists: false,
            footnotes: false,
            math: MathOptions::default(),
            mermaid: MermaidPolicy::Fenced,
            embedded_media: EmbeddedMediaPolicy::PreserveLink,
        }
    }
}

/// Extensibility options.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ExtensionOptions {
    pub custom_rules: Vec<CustomRule>,
    pub rule_packs: Vec<String>,
}

/// Central configuration structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConversionOptions {
    pub profile: OutputProfile,
    pub render: RenderOptions,
    pub cleanup: CleanupOptions,
    pub semantic: SemanticOptions,
    pub extension: ExtensionOptions,
    pub limits: Limits,
    /// Treat warnings as errors.
    pub strict: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            profile: OutputProfile::Commonmark,
            render: RenderOptions::default(),
            cleanup: CleanupOptions::default(),
            semantic: SemanticOptions::default(),
            extension: ExtensionOptions::default(),
            limits: Limits::default(),
            strict: false,
        }
    }
}

impl ConversionOptions {
    /// Create options for the GFM profile.
    pub fn gfm() -> Self {
        Self {
            profile: OutputProfile::Gfm,
            semantic: crate::options::SemanticOptions {
                task_lists: true,
                table_handling: TableHandling::Gfm,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Create options for the Extended profile.
    pub fn extended() -> Self {
        Self {
            profile: OutputProfile::Extended,
            semantic: crate::options::SemanticOptions {
                footnotes: true,
                definition_lists: true,
                math: MathOptions {
                    enabled: true,
                    output: MathOutput::InlineDollar,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Self::gfm()
        }
    }

    /// Create options for the Pandoc profile.
    pub fn pandoc() -> Self {
        let mut opts = Self::extended();
        opts.profile = OutputProfile::Pandoc;
        opts.render.raw_html_policy = RawHtmlPolicy::Faithful;
        opts.render.smart_punctuation = SmartPunctuation::Normalize;
        opts
    }

    /// Create options for the Obsidian profile.
    pub fn obsidian() -> Self {
        let mut opts = Self::extended();
        opts.profile = OutputProfile::Obsidian;
        opts.render.raw_html_policy = RawHtmlPolicy::Preserve;
        opts
    }

    /// Create options for the MDX-safe profile.
    pub fn mdx_safe() -> Self {
        let mut opts = Self::extended();
        opts.profile = OutputProfile::MdxSafe;
        opts.render.raw_html_policy = RawHtmlPolicy::Escape;
        opts.render.comment_policy = CommentPolicy::Drop;
        opts.render.escaping_mode = EscapingMode::Strict;
        opts
    }

    /// Create options for the plain-text profile.
    pub fn plain_text() -> Self {
        let mut opts = Self {
            profile: OutputProfile::PlainText,
            ..Default::default()
        };
        opts.render.raw_html_policy = RawHtmlPolicy::Drop;
        opts.cleanup.image_mode = ImageMode::AltText;
        opts
    }

    /// Apply profile-specific defaults when an option is still at its generic default.
    ///
    /// This is used by the CLI/config loader so that selecting a profile like
    /// `extended` or `obsidian` also enables the matching semantic features.
    pub fn apply_profile_defaults(&mut self) {
        let semantic_defaults = SemanticOptions::default();
        let render_defaults = RenderOptions::default();
        let cleanup_defaults = CleanupOptions::default();

        match self.profile {
            OutputProfile::Extended
            | OutputProfile::Pandoc
            | OutputProfile::Obsidian
            | OutputProfile::MdxSafe => {
                if self.semantic.footnotes == semantic_defaults.footnotes {
                    self.semantic.footnotes = true;
                }
                if self.semantic.definition_lists == semantic_defaults.definition_lists {
                    self.semantic.definition_lists = true;
                }
                if self.semantic.math == semantic_defaults.math {
                    self.semantic.math = MathOptions {
                        enabled: true,
                        output: MathOutput::InlineDollar,
                        ..Default::default()
                    };
                }
            }
            OutputProfile::Gfm => {
                if self.semantic.task_lists == semantic_defaults.task_lists {
                    self.semantic.task_lists = true;
                }
                if self.semantic.table_handling == semantic_defaults.table_handling {
                    self.semantic.table_handling = TableHandling::Gfm;
                }
            }
            _ => {}
        }

        if self.profile == OutputProfile::Pandoc {
            if self.render.raw_html_policy == render_defaults.raw_html_policy {
                self.render.raw_html_policy = RawHtmlPolicy::Faithful;
            }
            if self.render.smart_punctuation == render_defaults.smart_punctuation {
                self.render.smart_punctuation = SmartPunctuation::Normalize;
            }
        }

        if self.profile == OutputProfile::Obsidian
            && self.render.raw_html_policy == render_defaults.raw_html_policy
        {
            self.render.raw_html_policy = RawHtmlPolicy::Preserve;
        }

        if self.profile == OutputProfile::MdxSafe {
            if self.render.raw_html_policy == render_defaults.raw_html_policy {
                self.render.raw_html_policy = RawHtmlPolicy::Escape;
            }
            if self.render.comment_policy == render_defaults.comment_policy {
                self.render.comment_policy = CommentPolicy::Drop;
            }
            if self.render.escaping_mode == render_defaults.escaping_mode {
                self.render.escaping_mode = EscapingMode::Strict;
            }
        }

        if self.profile == OutputProfile::PlainText {
            if self.render.raw_html_policy == render_defaults.raw_html_policy {
                self.render.raw_html_policy = RawHtmlPolicy::Drop;
            }
            if self.cleanup.image_mode == cleanup_defaults.image_mode {
                self.cleanup.image_mode = ImageMode::AltText;
            }
        }
    }

    /// Validate the options value. Returns `Err` on the first configuration problem.
    pub fn validate(&self) -> crate::Result<()> {
        validation::validate(self)
    }
}

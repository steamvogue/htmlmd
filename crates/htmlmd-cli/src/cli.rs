// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Command-line interface for `htmlmd`.
#[derive(Debug, Parser)]
#[command(name = "htmlmd", version, about = "Convert HTML to Markdown", long_about = None)]
pub struct Cli {
    /// Input HTML files, directories, or glob patterns. Use `-` for stdin.
    /// Multiple files are processed in parallel.
    pub inputs: Vec<String>,

    /// Output file. Use `-` for stdout (default). Not allowed for multiple inputs
    /// unless `--output-dir` is given.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Output directory for batch conversions.
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<PathBuf>,

    /// Mirror the input directory tree under `--output-dir`.
    #[arg(long)]
    pub mirror: bool,

    /// Recurse into directories to find `.html` files.
    #[arg(short, long)]
    pub recursive: bool,

    /// Policy when an output file already exists.
    #[arg(long, value_enum, default_value = "overwrite")]
    pub output_policy: OutputPolicyArg,

    /// Write each output atomically (temp file + rename).
    #[arg(long)]
    pub atomic: bool,

    /// Preserve input file timestamps on output files.
    #[arg(long)]
    pub preserve_timestamps: bool,

    /// Write a JSON manifest mapping inputs to outputs, metadata and hashes.
    #[arg(long, value_name = "PATH")]
    pub manifest: Option<PathBuf>,

    /// Exit non-zero if any output would change. Implies reporting, no writes.
    #[arg(long)]
    pub check: bool,

    /// Print a diff when `--check` detects changes.
    #[arg(long)]
    pub diff: bool,

    /// Explicit input encoding (e.g. `utf-8`, `windows-1252`).
    #[arg(long)]
    pub encoding: Option<String>,

    /// Configuration file (TOML or JSON).
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Output profile.
    #[arg(long, value_enum)]
    pub profile: Option<ProfileArg>,

    /// Heading style.
    #[arg(long, value_enum)]
    pub heading_style: Option<HeadingStyleArg>,

    /// Bullet list marker.
    #[arg(long, value_enum)]
    pub bullet: Option<BulletArg>,

    /// Link style.
    #[arg(long, value_enum)]
    pub link_style: Option<LinkStyleArg>,

    /// Reference link/image definition placement.
    #[arg(long, value_enum)]
    pub reference_placement: Option<ReferencePlacementArg>,

    /// Image rendering mode.
    #[arg(long, value_enum)]
    pub image_mode: Option<ImageModeArg>,

    /// Code fence delimiter.
    #[arg(long, value_enum)]
    pub code_fence: Option<CodeFenceArg>,

    /// Horizontal rule style.
    #[arg(long, value_enum)]
    pub hr_style: Option<HrStyleArg>,

    /// Hard line break style.
    #[arg(long, value_enum)]
    pub br_style: Option<BrStyleArg>,

    /// HTML tags to skip (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub skip_tags: Option<Vec<String>>,

    /// CSS selectors to remove (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub remove_selectors: Option<Vec<String>>,

    /// CSS selectors to unwrap (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub unwrap_selectors: Option<Vec<String>>,

    /// Keep only content matched by these selectors (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub keep_only_selectors: Option<Vec<String>>,

    /// Extract only the first element matching this selector.
    #[arg(long)]
    pub extract_selector: Option<String>,

    /// Base URL for resolving relative links and images.
    #[arg(long)]
    pub base_url: Option<String>,

    /// Remove common tracking parameters from URLs.
    #[arg(long, default_missing_value = "true", num_args = 0..=1)]
    pub remove_tracking_params: Option<bool>,

    /// Strict mode: turn warnings into errors.
    #[arg(long)]
    pub strict: bool,

    /// Extract the document title from `<title>`.
    #[arg(long)]
    pub metadata_title: bool,

    /// Extract the document description from `meta[name="description"]`.
    #[arg(long)]
    pub metadata_description: bool,

    /// Extract the canonical URL from `link[rel="canonical"]`.
    #[arg(long)]
    pub metadata_canonical_url: bool,

    /// Number of parallel jobs. 0 means use all CPUs.
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Print the default configuration and exit.
    #[arg(long)]
    pub print_default_config: bool,

    /// Print the effective configuration and exit.
    #[arg(long)]
    pub print_effective_config: bool,

    /// Simulate conversion without writing output.
    #[arg(long)]
    pub dry_run: bool,

    /// Fold non-breaking spaces (U+00A0, U+2007, U+202F) to regular spaces.
    #[arg(long)]
    pub normalize_whitespace: bool,

    /// Suppress non-error output.
    #[arg(short, long)]
    pub quiet: bool,

    /// Increase verbosity (repeatable).
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputPolicyArg {
    Overwrite,
    SkipExisting,
    FailIfExists,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProfileArg {
    Commonmark,
    Gfm,
    Extended,
    Pandoc,
    Obsidian,
    MdxSafe,
    PlainText,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HeadingStyleArg {
    Atx,
    Setex,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BulletArg {
    Hyphen,
    Asterisk,
    Plus,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LinkStyleArg {
    Inline,
    Reference,
    CollapsedReference,
    ShortcutReference,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReferencePlacementArg {
    End,
    SectionEnd,
    Adjacent,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ImageModeArg {
    Inline,
    Reference,
    Skip,
    AltText,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CodeFenceArg {
    Backticks,
    Tildes,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum HrStyleArg {
    Dashes,
    Asterisks,
    Underscores,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BrStyleArg {
    TwoSpaces,
    Backslash,
}

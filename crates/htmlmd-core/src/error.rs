// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io;
use thiserror::Error;

/// Errors that can occur during HTML to Markdown conversion.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// I/O error while reading or writing.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// HTML parsing failed.
    #[error("HTML parse error: {0}")]
    Parse(String),

    /// URL parsing or resolution failed.
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Invalid CSS selector.
    #[error("selector error: {0}")]
    Selector(String),

    /// Configuration loading or validation failed.
    #[error("configuration error: {0}")]
    Config(String),

    /// A configured limit was exceeded.
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),

    /// Catch-all for unexpected conditions.
    #[error("{0}")]
    Other(String),
}

/// Result type alias used throughout `htmlmd-core`.
pub type Result<T> = std::result::Result<T, Error>;

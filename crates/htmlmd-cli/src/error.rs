// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("conversion error: {0}")]
    Conversion(#[from] htmlmd_core::Error),

    #[error("output path required for multiple inputs")]
    OutputRequired,
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Config(_) | CliError::OutputRequired => 2,
            CliError::Conversion(_) | CliError::Io(_) => 1,
        }
    }
}

impl From<glob::PatternError> for CliError {
    fn from(e: glob::PatternError) -> Self {
        CliError::Config(format!("invalid glob pattern: {e}"))
    }
}

impl From<glob::GlobError> for CliError {
    fn from(e: glob::GlobError) -> Self {
        CliError::Io(io::Error::other(e))
    }
}

impl From<serde_json::Error> for CliError {
    fn from(e: serde_json::Error) -> Self {
        CliError::Config(format!("JSON error: {e}"))
    }
}

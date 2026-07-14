// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::diagnostic::Diagnostic;

/// The result of converting a single HTML document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConversionResult {
    /// Converted Markdown output.
    pub markdown: String,
    /// Document title, if extracted.
    pub title: Option<String>,
    /// Document description, if extracted.
    pub description: Option<String>,
    /// Canonical URL, if extracted.
    pub canonical_url: Option<String>,
    /// Diagnostics collected during conversion.
    pub diagnostics: Vec<Diagnostic>,
}

impl ConversionResult {
    /// Return true if any diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.kind, super::diagnostic::DiagnosticKind::Error))
    }
}
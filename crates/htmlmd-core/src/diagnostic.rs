// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};

/// Classification of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticKind {
    Warning,
    Error,
}

/// A single diagnostic message produced during conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub path: Option<String>,
    pub selector: Option<String>,
    pub rule: Option<String>,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            kind: DiagnosticKind::Warning,
            message: message.into(),
            path: None,
            selector: None,
            rule: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            kind: DiagnosticKind::Error,
            message: message.into(),
            path: None,
            selector: None,
            rule: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }

    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }
}

/// Trait for collecting diagnostics.
pub trait DiagnosticsCollector: Send + Sync {
    fn push(&mut self, diagnostic: Diagnostic);
}

impl DiagnosticsCollector for Vec<Diagnostic> {
    fn push(&mut self, diagnostic: Diagnostic) {
        Vec::push(self, diagnostic);
    }
}

impl DiagnosticsCollector for () {
    fn push(&mut self, _diagnostic: Diagnostic) {}
}

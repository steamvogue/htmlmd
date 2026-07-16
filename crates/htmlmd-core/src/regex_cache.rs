// SPDX-License-Identifier: MIT OR Apache-2.0

//! Process-wide cache for user-configured regex patterns.
//!
//! Patterns come from configuration (language classes, URL rewrite rules), so
//! the set is small and stable across a batch run; caching makes their compile
//! cost one-time per process instead of per conversion. `Regex` clones share
//! the compiled program internally, so handing out clones is cheap.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use regex::Regex;

static CACHE: Lazy<Mutex<HashMap<String, Option<Regex>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Compile `pattern`, caching the result (including failures) process-wide.
///
/// Returns `None` for invalid patterns; those are rejected up front by
/// `ConversionOptions::validate`, so hitting `None` here just skips the rule.
pub(crate) fn cached_regex(pattern: &str) -> Option<Regex> {
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).ok())
        .clone()
}

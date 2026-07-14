// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::diagnostic::DiagnosticsCollector;
use crate::options::ConversionOptions;

/// Rewrite a single URL attribute value.
///
/// For `srcset`, only the first URL is rewritten according to the responsive
/// image policy. In Phase 1 this is a simple first-candidate rewrite.
pub fn rewrite_url_attr(
    value: &str,
    attr_name: &str,
    base: Option<&url::Url>,
    options: &ConversionOptions,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> String {
    if attr_name.eq_ignore_ascii_case("srcset") {
        rewrite_srcset(value, base, options, diagnostics)
    } else {
        rewrite_single_url(value, base, options, diagnostics)
    }
}

fn rewrite_single_url(
    url: &str,
    base: Option<&url::Url>,
    options: &ConversionOptions,
    _diagnostics: &mut dyn DiagnosticsCollector,
) -> String {
    // 1. Security checks.
    if let Some(scheme) = url.split(':').next() {
        let scheme = scheme.to_lowercase();
        if options.cleanup.blocked_url_schemes.contains(&scheme)
            && !options.cleanup.allowed_url_schemes.contains(&scheme)
        {
            return String::new();
        }
    }

    // 2. Resolve relative URLs.
    let resolved = if let Some(base) = base {
        base.join(url).map(|u| u.to_string()).unwrap_or_else(|_| url.to_string())
    } else {
        url.to_string()
    };

    // 3. Rewrite via configured rules.
    let mut rewritten = resolved;
    for rule in &options.cleanup.url_rewrite_rules {
        if let Ok(re) = regex::Regex::new(&rule.pattern) {
            rewritten = re.replace_all(&rewritten, rule.replacement.as_str()).to_string();
        }
    }

    // 4. Strip tracking parameters.
    if options.cleanup.remove_tracking_params {
        rewritten = strip_tracking_params(&rewritten, options);
    }

    rewritten
}

fn rewrite_srcset(
    value: &str,
    base: Option<&url::Url>,
    options: &ConversionOptions,
    diagnostics: &mut dyn DiagnosticsCollector,
) -> String {
    let candidates: Vec<&str> = value.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if candidates.is_empty() {
        return value.to_string();
    }

    let chosen = match options.cleanup.responsive_image_policy {
        crate::options::ResponsiveImagePolicy::FirstSrcset => candidates[0],
        crate::options::ResponsiveImagePolicy::Largest => choose_largest_srcset(&candidates),
        crate::options::ResponsiveImagePolicy::PreserveSrcset => candidates[0],
    };

    let url_part = chosen.split_whitespace().next().unwrap_or(chosen);
    rewrite_single_url(url_part, base, options, diagnostics)
}

fn choose_largest_srcset<'a>(candidates: &'a [&'a str]) -> &'a str {
    candidates
        .iter()
        .max_by_key(|c| {
            c.split_whitespace()
                .nth(1)
                .and_then(|d| {
                    if let Some(n) = d.strip_suffix('w') {
                        n.parse::<u32>().ok()
                    } else {
                        d.strip_suffix('x').map(|n| (n.parse::<f32>().unwrap_or(0.0) * 1000.0) as u32)
                    }
                })
                .unwrap_or(0)
        })
        .copied()
        .unwrap_or(candidates[0])
}

fn strip_tracking_params(url: &str, options: &ConversionOptions) -> String {
    let Ok(parsed) = url::Url::parse(url) else {
        return url.to_string();
    };

    let default_tracking = ["utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content", "fbclid", "gclid"];
    let custom: Vec<&str> = options.cleanup.tracking_params.iter().map(|s| s.as_str()).collect();

    let query: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| {
            let key = k.as_ref();
            !default_tracking.contains(&key) && !custom.contains(&key)
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let mut out = parsed;
    out.set_query(None);
    if !query.is_empty() {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in &query {
            serializer.append_pair(k, v);
        }
        out.set_query(Some(&serializer.finish()));
    }
    out.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;
    use crate::options::ConversionOptions;

    fn collector() -> Vec<Diagnostic> {
        Vec::new()
    }

    #[test]
    fn strips_utm_params() {
        let options = ConversionOptions::default();
        let mut d = collector();
        let out = rewrite_single_url(
            "https://example.com/page?utm_source=email&utm_campaign=x",
            None,
            &options,
            &mut d,
        );
        assert_eq!(out, "https://example.com/page");
    }

    #[test]
    fn resolves_relative_url() {
        let options = ConversionOptions::default();
        let base = url::Url::parse("https://example.com/blog/").unwrap();
        let mut d = collector();
        let out = rewrite_single_url("../img.png", Some(&base), &options, &mut d);
        assert_eq!(out, "https://example.com/img.png");
    }

    #[test]
    fn blocks_javascript_url() {
        let options = ConversionOptions::default();
        let mut d = collector();
        let out = rewrite_single_url("javascript:alert(1)", None, &options, &mut d);
        assert!(out.is_empty());
    }
}

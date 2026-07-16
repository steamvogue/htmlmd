// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::backend::ConverterBackend;
use crate::error::Result;
use crate::htmd_handlers;
use crate::options::ConversionOptions;
use crate::result::ConversionResult;

/// The `htmd`-based converter backend.
#[derive(Debug, Default, Clone, Copy)]
pub struct HtmdBackend;

impl HtmdBackend {
    pub fn new() -> Self {
        Self
    }
}

impl ConverterBackend for HtmdBackend {
    fn convert(&self, html: &str, options: &ConversionOptions) -> Result<ConversionResult> {
        let markdown = htmd_handlers::convert_with_htmd(html, options)?;
        let markdown = crate::postprocess::post_process(&markdown, options);

        Ok(ConversionResult {
            markdown,
            title: None,
            description: None,
            canonical_url: None,
            diagnostics: Vec::new(),
        })
    }
}

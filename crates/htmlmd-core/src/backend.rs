// SPDX-License-Identifier: MIT OR Apache-2.0

use std::io::Write;

use crate::error::Result;
use crate::options::ConversionOptions;
use crate::result::ConversionResult;

/// Internal trait abstracting the HTML-to-Markdown translation backend.
///
/// This hides the concrete converter (currently `htmd`) so it can be replaced
/// or augmented in future phases without changing the public `htmlmd-core` API.
pub trait ConverterBackend: Send + Sync {
    /// Convert a UTF-8 HTML string to Markdown.
    fn convert(&self, html: &str, options: &ConversionOptions) -> Result<ConversionResult>;

    /// Convert an already-parsed (cleaned) document. Default serializes and
    /// delegates to `convert`, preserving old backends' behavior.
    fn convert_dom(
        &self,
        document: &scraper::Html,
        options: &ConversionOptions,
    ) -> Result<ConversionResult> {
        self.convert(&document.html(), options)
    }

    /// Convert and write the Markdown output directly to a writer.
    fn convert_to_writer(
        &self,
        html: &str,
        options: &ConversionOptions,
        writer: &mut dyn Write,
    ) -> Result<()> {
        let result = self.convert(html, options)?;
        writer.write_all(result.markdown.as_bytes())?;
        Ok(())
    }
}

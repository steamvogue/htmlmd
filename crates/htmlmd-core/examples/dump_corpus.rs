// SPDX-License-Identifier: MIT OR Apache-2.0

//! Write the synthetic benchmark corpus to files so external tools
//! (turndown, markdownify, html2markdown, pandoc, …) can be benchmarked on
//! byte-identical input. Used by benches/compare/run.sh.
//!
//! Usage: cargo run --release -p htmlmd-core --example dump_corpus -- <out-dir>

use htmlmd_core::corpus::{
    generate_docs_doc, generate_news_doc, generate_table_doc, generate_wiki_doc,
};

fn main() -> std::io::Result<()> {
    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "benches/compare/corpus".to_string());
    std::fs::create_dir_all(&out_dir)?;

    for (name, html) in [
        ("wiki", generate_wiki_doc()),
        ("news", generate_news_doc()),
        ("docs", generate_docs_doc()),
        ("tables", generate_table_doc()),
    ] {
        let path = format!("{out_dir}/{name}.html");
        std::fs::write(&path, &html)?;
        println!("{path}\t{} bytes", html.len());
    }
    Ok(())
}

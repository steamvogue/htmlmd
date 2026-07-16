// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::Element;
use crate::native::element_handler::element_util::{serialize_element, serialize_if_faithful};
use crate::native::element_handler::{HandlerResult, Handlers};
use crate::native::node_util::{get_node_children, get_node_tag_name, get_parent_node};
use crate::native::options::TranslationMode;
use crate::native::text_util::{TrimDocumentWhitespace, concat_strings};
use ego_tree::NodeRef;
use scraper::Node;

/// Handler for table elements.
///
/// Converts HTML tables to Markdown tables using the pipe syntax:
/// ```text
/// | Header1 | Header2 |
/// | ------- | ------- |
/// | Cell1   | Cell2   |
/// ```
pub(crate) fn table_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    if handlers.options().translation_mode == TranslationMode::Pure
        && (!has_explicit_headers(element.node) || is_inside_table_cell(element.node))
    {
        return handlers.fallback(element);
    }

    // All child table elements must be markdown translated to markdown
    // translate the table in faithful mode.
    // We track markdown translation status manually because we iterate children
    let mut all_children_translated = true;

    // Extract table rows
    let mut captions: Vec<String> = Vec::new();
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_thead = false;

    // Extract rows and headers from the table structure
    if element.node.value().is_element() {
        for child in get_node_children(element.node) {
            if let Node::Element(child_element) = child.value() {
                let tag_name = child_element.name.local.as_ref();

                match tag_name {
                    "caption" => {
                        if let Some(res) = handlers.handle(child) {
                            captions.push(res.content.trim_document_whitespace().to_string());
                        }
                    }
                    "thead" => {
                        let tr = child
                            .children()
                            .find(|it| get_node_tag_name(*it).is_some_and(|tag| tag == "tr"));

                        let row_node = match tr {
                            Some(tr) => tr,
                            None => child,
                        };

                        has_thead = true;
                        let (cells, translated) = extract_row_cells(handlers, row_node, "th");
                        headers = cells;
                        all_children_translated &= translated;
                        if headers.is_empty() {
                            let (cells, translated) = extract_row_cells(handlers, row_node, "td");
                            headers = cells;
                            all_children_translated &= translated;
                        }
                    }
                    "tbody" | "tfoot" => {
                        for row_node in get_node_children(child) {
                            let is_tr = match row_node.value() {
                                Node::Element(row_element) => {
                                    row_element.name.local.as_ref() == "tr"
                                }
                                _ => false,
                            };
                            if is_tr {
                                // If no thead is found, use the first th row as header
                                if !has_thead && headers.is_empty() {
                                    let (cells, translated) =
                                        extract_row_cells(handlers, row_node, "th");
                                    headers = cells;
                                    all_children_translated &= translated;
                                    has_thead = !headers.is_empty();

                                    if has_thead {
                                        continue;
                                    }
                                }

                                let (row_cells, translated) =
                                    extract_row_cells(handlers, row_node, "td");
                                all_children_translated &= translated;
                                if !row_cells.is_empty() {
                                    rows.push(row_cells);
                                }
                            }
                        }
                    }
                    "tr" => {
                        // If no thead is found, use the first row as headers
                        if !has_thead && headers.is_empty() {
                            let (cells, translated) = extract_row_cells(handlers, child, "th");
                            headers = cells;
                            all_children_translated &= translated;
                            if headers.is_empty() {
                                let (cells, translated) = extract_row_cells(handlers, child, "td");
                                if !cells.is_empty() {
                                    headers = cells;
                                    all_children_translated &= translated;
                                }
                            }
                            has_thead = !headers.is_empty();
                        } else {
                            let (row_cells, translated) = extract_row_cells(handlers, child, "td");
                            all_children_translated &= translated;
                            if !row_cells.is_empty() {
                                rows.push(row_cells);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if handlers.options().translation_mode == TranslationMode::Faithful && !all_children_translated
    {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }

    // If we didn't find any rows or cells, just return the content as-is
    if rows.is_empty() && headers.is_empty() {
        let content = handlers.walk_children(element.node).content;
        let content = content.trim_matches('\n');
        if content.is_empty() {
            return None;
        }
        return Some(concat_strings!("\n\n", content, "\n\n").into());
    }

    // Determine the number of columns by finding the max column count
    let num_columns = if headers.is_empty() {
        rows.iter().map(|row| row.len()).max().unwrap_or(0)
    } else {
        headers.len()
    };

    if num_columns == 0 {
        let content = handlers.walk_children(element.node).content;
        let content = content.trim_matches('\n');
        if content.is_empty() {
            return None;
        }
        return Some(concat_strings!("\n\n", content, "\n\n").into());
    }

    // Build the Markdown table
    let mut table_md = String::from("\n\n");

    for caption in captions {
        table_md.push_str(&format!("{caption}\n"));
    }

    let col_widths = compute_column_widths(&headers, &rows, num_columns);

    if !headers.is_empty() {
        table_md.push_str(&format_row_padded(&headers, num_columns, &col_widths));
        table_md.push_str(&format_separator_padded(num_columns, &col_widths));
    }
    for row in rows {
        table_md.push_str(&format_row_padded(&row, num_columns, &col_widths));
    }

    table_md.push('\n');
    Some(table_md.into())
}

fn has_explicit_headers(node: NodeRef<'_, Node>) -> bool {
    fn visit(node: NodeRef<'_, Node>, is_root: bool) -> bool {
        for child in get_node_children(node) {
            if let Node::Element(child_element) = child.value() {
                let tag_name = child_element.name.local.as_ref();
                if !is_root && tag_name == "table" {
                    continue;
                }
                if matches!(tag_name, "th" | "thead") {
                    return true;
                }
            }

            if visit(child, false) {
                return true;
            }
        }

        false
    }

    visit(node, true)
}

fn is_inside_table_cell(node: NodeRef<'_, Node>) -> bool {
    let mut current = get_parent_node(node);

    while let Some(parent) = current {
        if get_node_tag_name(parent).is_some_and(|tag| matches!(tag, "td" | "th")) {
            return true;
        }
        current = get_parent_node(parent);
    }

    false
}

/// Extract cells from a row node
fn extract_row_cells(
    handlers: &dyn Handlers,
    row_node: NodeRef<'_, Node>,
    cell_tag: &str,
) -> (Vec<String>, bool) {
    let mut cells = Vec::new();
    let mut all_translated = true;

    for cell_node in get_node_children(row_node) {
        let is_cell = match cell_node.value() {
            Node::Element(cell_element) => cell_element.name.local.as_ref() == cell_tag,
            _ => false,
        };
        if is_cell {
            let Some(res) = handlers.handle(cell_node) else {
                continue;
            };
            if !res.markdown_translated {
                all_translated = false;
            }
            let cell_content = res.content.trim_document_whitespace().to_string();
            cells.push(cell_content);
        }
    }

    (cells, all_translated)
}

/// Normalize cell content for Markdown table representation
fn normalize_cell_content(content: &str) -> String {
    let content = content
        .replace('\n', " ")
        .replace('\r', "")
        .replace('|', "&#124;");
    content.trim_document_whitespace().to_string()
}

fn format_row_padded(row: &[String], num_columns: usize, col_widths: &[usize]) -> String {
    let mut line = String::from("|");
    for (i, col_width) in col_widths.iter().enumerate().take(num_columns) {
        let cell = row
            .get(i)
            .map(|s| normalize_cell_content(s))
            .unwrap_or_default();
        let pad = col_width.saturating_sub(cell.chars().count());
        line.push_str(&concat_strings!(" ", cell, " ".repeat(pad), " |"));
    }
    line.push('\n');
    line
}

fn format_separator_padded(num_columns: usize, col_widths: &[usize]) -> String {
    let mut line = String::from("|");
    for col_width in col_widths.iter().take(num_columns) {
        line.push_str(&concat_strings!(" ", "-".repeat(*col_width), " |"));
    }
    line.push('\n');
    line
}

fn compute_column_widths(
    headers: &[String],
    rows: &[Vec<String>],
    num_columns: usize,
) -> Vec<usize> {
    let mut widths = vec![0; num_columns];
    for (i, header) in headers.iter().enumerate() {
        widths[i] = header.chars().count();
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(num_columns) {
            let len = cell.chars().count();
            if len > widths[i] {
                widths[i] = len;
            }
        }
    }
    widths
}

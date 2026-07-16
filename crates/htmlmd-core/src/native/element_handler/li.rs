// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{HandlerResult, Handlers},
    node_util::{get_node_tag_name, get_parent_node},
    options::BulletListMarker,
    text_util::{TrimDocumentWhitespace, concat_strings, indent_text_except_first_line},
};

pub(super) fn list_item_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers
        .walk_children(element.node)
        .content
        .trim_start_document_whitespace()
        .to_string();

    let ul_li = || {
        let marker = if handlers.options().bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        let spacing = " ".repeat(handlers.options().ul_bullet_spacing.into());
        let content = indent_text_except_first_line(&content, marker.len() + spacing.len(), true);

        Some(concat_strings!("\n", marker, spacing, content).into())
    };

    let ol_li = || {
        // Marker will be added in the ol handler
        Some(concat_strings!("\n", content, "\n").into())
    };

    let is_parent_ol = get_parent_node(element.node)
        .and_then(get_node_tag_name)
        .is_some_and(|parent_tag_name| parent_tag_name == "ol");
    if is_parent_ol { ol_li() } else { ul_li() }
}

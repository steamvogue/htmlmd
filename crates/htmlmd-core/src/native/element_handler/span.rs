// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{HandlerResult, Handlers},
    text_util::concat_strings,
};

pub(super) fn span_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // See if this contains math: `<span class="math math-inline/display>text-only content</span>`.
    if element.attrs.len() == 1 {
        let (name, value) = &element.attrs[0];
        if name.local.as_ref() == "class" {
            let mut children = element.node.children();
            let only_child = children.next().filter(|_| children.next().is_none());
            // `text_of` is `Some` only for text nodes, matching htmd's
            // `NodeData::Text` requirement (and honoring the combined-text
            // side table, since htmd reads the merged rcdom contents here).
            if let Some(contents) = only_child.and_then(|child| handlers.text_of(child)) {
                if value.as_ref() == "math math-inline" {
                    return Some(concat_strings!("$", contents, "$").into());
                }

                if value.as_ref() == "math math-display" {
                    return Some(concat_strings!("$$", contents, "$$").into());
                }
            }
        }
    }

    // Always serialize as HTML if we're in faithful mode.
    serialize_if_faithful!(handlers, element, -1);

    // Otherwise, just return the contents.
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_matches('\n');

    Some(content.into())
}

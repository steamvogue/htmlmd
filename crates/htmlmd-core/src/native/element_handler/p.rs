// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{HandlerResult, Handlers},
    text_util::concat_strings,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_matches('\n');
    Some(concat_strings!("\n\n", content, "\n\n").into())
}

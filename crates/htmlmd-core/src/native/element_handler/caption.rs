// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn caption_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    handle_or_serialize_by_parent(handlers, &element, &["table"], element.markdown_translated)
}

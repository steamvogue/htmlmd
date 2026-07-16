// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::handle_or_serialize_by_parent,
    element_handler::{HandlerResult, Handlers},
};

pub(super) fn head_body_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    handle_or_serialize_by_parent(handlers, &element, &["html"], true)
}

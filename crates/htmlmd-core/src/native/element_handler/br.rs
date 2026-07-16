// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{HandlerResult, Handlers},
    options::BrStyle,
};

pub(super) fn br_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);

    match handlers.options().br_style {
        BrStyle::TwoSpaces => Some("  \n".into()),
        BrStyle::Backslash => Some("\\\n".into()),
    }
}

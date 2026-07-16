// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

use crate::native::{
    Element,
    element_handler::element_util::serialize_if_faithful,
    element_handler::{HandlerResult, Handlers},
    options::HrStyle,
};

pub(super) fn hr_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    match handlers.options().hr_style {
        HrStyle::Dashes => Some("\n\n- - -\n\n".into()),
        HrStyle::Asterisks => Some("\n\n* * *\n\n".into()),
        HrStyle::Underscores => Some("\n\n_ _ _\n\n".into()),
    }
}

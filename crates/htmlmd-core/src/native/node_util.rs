// SPDX-License-Identifier: Apache-2.0
// Portions adapted from htmd v0.5.4 (https://github.com/letmutex/htmd), © letmutex, Apache-2.0.

//! Node helpers ported from htmd's `node_util.rs`, mapped from
//! `markup5ever_rcdom` (`Rc<Node>`, `RefCell` children/parent links) to
//! scraper's `ego_tree` (`NodeRef`, which is `Copy` and navigates the arena
//! directly).

use ego_tree::NodeRef;
use scraper::Node;

/// rcdom's `NodeData::Document` maps to "html" in htmd; scraper additionally
/// has a `Fragment` root which we treat the same way.
pub(crate) fn get_node_tag_name<'a>(node: NodeRef<'a, Node>) -> Option<&'a str> {
    match node.value() {
        Node::Document | Node::Fragment => Some("html"),
        Node::Element(element) => Some(element.name.local.as_ref()),
        _ => None,
    }
}

/// htmd's `get_parent_node` juggles rcdom's `Cell<Option<WeakHandle>>`;
/// ego-tree stores parent links in the arena, so this is a direct call.
pub(crate) fn get_parent_node<'a>(node: NodeRef<'a, Node>) -> Option<NodeRef<'a, Node>> {
    node.parent()
}

// Check to see if node's parent's tag name matches the provided string.
pub(crate) fn parent_tag_name_equals(node: NodeRef<'_, Node>, tag_names: &[&str]) -> bool {
    if let Some(parent) = get_parent_node(node) {
        if let Some(actual_tag_name) = get_node_tag_name(parent) {
            return tag_names.contains(&actual_tag_name);
        }
    }
    false
}

pub(crate) fn get_node_children<'a>(node: NodeRef<'a, Node>) -> Vec<NodeRef<'a, Node>> {
    node.children().collect()
}

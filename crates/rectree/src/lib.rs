#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use core::fmt::{Display, Formatter};
use core::ops::Deref;

use alloc::vec;
use hashbrown::HashSet;

use crate::node::RectNode;
use crate::sparse_map::{Key, SparseMap};

pub use kurbo;

pub mod layout;
pub mod mut_detect;
pub mod node;
pub mod sparse_map;

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct NodeId(Key);

impl Deref for NodeId {
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("NodeId({})", self.0))
    }
}

/// A hierarchical tree of rectangular layout nodes.
///
/// `Rectree` maintains parent–child relationships between [`RectNode`]s,
/// supports multiple root nodes, and provides stable [`NodeId`]s for
/// insertion, lookup, and removal.
///
/// The tree owns all nodes and ensures structural consistency when
/// inserting or removing subtrees.
#[derive(Default, Debug)]
pub struct Rectree {
    /// Identifiers of all root nodes (nodes without a parent).
    root_ids: HashSet<NodeId>,
    /// Storage for all nodes in the tree, indexed by [`NodeId`].
    ///
    /// This uses a sparse map to provide stable identifiers while
    /// allowing efficient insertion and removal.
    nodes: SparseMap<RectNode>,
}

impl Rectree {
    /// Creates an empty [`Rectree`].
    ///
    /// This is equivalent to calling [`Default::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a node into the tree while keeping track of the
    /// parent-child relationship.
    ///
    /// # Panics
    ///
    /// Panics if an invalid parent [`NodeId`] is used.
    pub fn insert(&mut self, mut node: RectNode) -> NodeId {
        let key = self.nodes.insert_with_key(|nodes, key| {
            let id = NodeId(key);
            if let Some(parent) = node.parent {
                let parent_node =
                    nodes.get_mut(&parent).unwrap_or_else(|| {
                        panic!("Invalid parent Id ({parent}).")
                    });

                parent_node.children.insert(id);
                node.depth = parent_node.depth + 1;
            } else {
                // No parent, meaning that it's a root id.
                self.root_ids.insert(id);
            }

            node
        });

        NodeId(key)
    }

    /// Removes a node and all of its descendants from the tree.
    ///
    /// Returns `true` if the node existed and was removed, or `false`
    /// if the given [`NodeId`] does not exist.
    pub fn remove(&mut self, id: &NodeId) -> bool {
        if let Some(node) = self.nodes.get(id) {
            if let Some(parent) =
                node.parent.and_then(|id| self.nodes.get_mut(&id))
            {
                parent.children.remove(id);
            }

            self.remove_recursive(id);
            return true;
        }

        false
    }

    // TODO: RectreeFragment (below).
    // TODO: Support detach node -> fragment.
    // TODO: Support attach fragment.

    /// Recursively removes a node and all of its descendants.
    ///
    /// This is an internal helper used by [`Self::remove()`].
    /// It assumes that any necessary parent bookkeeping has already
    /// been handled.
    fn remove_recursive(&mut self, id: &NodeId) {
        let mut child_stack = vec![*id];

        while let Some(id) = child_stack.pop() {
            let node = self.get(&id);

            child_stack.extend(node.children());
            self.nodes.remove(&id);
        }
    }

    /// Returns an immutable reference to a node if it exists.
    pub fn try_get(&self, id: &NodeId) -> Option<&RectNode> {
        self.nodes.get(id)
    }

    /// Returns a mutable reference to a node if it exists.
    pub fn try_get_mut(
        &mut self,
        id: &NodeId,
    ) -> Option<&mut RectNode> {
        self.nodes.get_mut(id)
    }

    /// Returns an immutable reference to a node.
    ///
    /// # Panics
    ///
    /// Panics if the given [`NodeId`] does not exist in the tree.
    pub fn get(&self, id: &NodeId) -> &RectNode {
        self.try_get(id).unwrap_or_else(|| {
            panic!("{id} does not exists in tree.")
        })
    }

    /// Returns a mutable reference to a node.
    ///
    /// # Panics
    ///
    /// Panics if the given [`NodeId`] does not exist in the tree.
    pub fn get_mut(&mut self, id: &NodeId) -> &mut RectNode {
        self.try_get_mut(id).unwrap_or_else(|| {
            panic!("{id} does not exists in tree.")
        })
    }

    /// Returns the set of root node identifiers.
    ///
    /// Root nodes are nodes that do not have a parent.
    pub fn root_ids(&self) -> &HashSet<NodeId> {
        &self.root_ids
    }
}

// #[derive(Debug)]
// pub struct RectreeFragment {
//     root_ids: NodeId,
//     nodes: SparseMap<RectNode>,
// }
//
// impl RectreeFragment {
//     pub fn into_tree() -> Rectree {}
// }

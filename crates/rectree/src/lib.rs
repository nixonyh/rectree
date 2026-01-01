#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use core::fmt::{Display, Formatter};
use core::ops::Deref;

use alloc::collections::btree_set::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashSet;
use kurbo::{Size, Vec2};

use crate::node::RectNode;
use crate::sparse_map::{Key, SparseMap};

pub use kurbo;

pub mod mut_detect;
pub mod node;
pub mod sparse_map;
// pub mod vec2;

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

// #[derive(Debug)]
// pub struct RectreeFragment {
//     root_ids: NodeId,
//     nodes: SparseMap<RectNode>,
// }
//
// impl RectreeFragment {
//     pub fn into_tree() -> Rectree {}
// }

#[derive(Default, Debug)]
pub struct Rectree {
    root_ids: HashSet<NodeId>,
    nodes: SparseMap<RectNode>,
}

// TODO: Separate states from the tree data into "contexts".
pub struct LayoutCtx<'a> {
    tree: &'a mut Rectree,
    scheduled_relayout: BTreeSet<DepthNode>, // Should be moved from `EditCtx`.
}

impl<'a> LayoutCtx<'a> {
    pub fn new(tree: &'a mut Rectree) -> Self {
        Self {
            tree,
            scheduled_relayout: BTreeSet::new(),
        }
    }

    pub fn schedule_relayout(&mut self, id: NodeId) -> bool {
        if let Some(node) = self.tree.try_get(&id) {
            return self
                .scheduled_relayout
                .insert(DepthNode::new(node.depth, id));
        }

        false
    }

    pub fn layout<L>(mut self, layouter: &L)
    where
        L: Layouter,
    {
        // Initialize reusable heap allocations.
        let mut visited_nodes = HashSet::<NodeId>::new(); // TODO: Could we avoid using a hashmap?
        // (node, constraint index) stack pending for layout build.
        let mut rebuild_stack = Vec::<(NodeId, usize)>::new();
        let mut child_stack = Vec::<NodeId>::new();
        let mut constraint_stack = Vec::<Constraint>::new();
        let mut new_translations = Vec::<(NodeId, Vec2)>::new();
        let mut mutated_translations =
            self.scheduled_relayout.clone();

        // Pop the deepest nodes first to ensure children are finalized before parents.
        while let Some(DepthNode { id, .. }) =
            self.scheduled_relayout.pop_last()
        {
            rebuild_stack.clear();
            child_stack.clear();
            constraint_stack.clear();

            let node = self.tree.get(&id);
            let initial_size = node.size;

            rebuild_stack.push((id, 0));
            child_stack.extend(node.children());
            constraint_stack.push(node.constraint);

            // Traverse the tree and create the build stack.
            while let Some(id) = child_stack.pop() {
                // Skip visited nodes, visited nodes should not be rebuilt.
                if visited_nodes.contains(&id) {
                    continue;
                }

                let node = self.tree.get(&id);
                let constraint = layouter.constraint(&id, self.tree);
                let constraint_index = constraint_stack.len();
                constraint_stack.push(constraint);

                for child in node.children() {
                    // Nothing to rebuild if the constraint is still the same.
                    if node.constraint != constraint {
                        // node.constraint = constraint;
                        rebuild_stack
                            .push((*child, constraint_index));

                        // Continue down the child tree.
                        child_stack.push(*child);
                    }
                }
            }

            // Build out the size and position the children.
            for (id, constraint_index) in
                rebuild_stack.drain(..).rev()
            {
                new_translations.clear();

                let set_translation =
                    |id: NodeId, translation: Vec2| {
                        new_translations.push((id, translation));
                    };

                self.tree.get_mut(&id).constraint =
                    constraint_stack[constraint_index];

                // Build size.
                let size =
                    layouter.build(&id, self.tree, set_translation);

                // Update translation.
                for (id, translation) in new_translations.drain(..) {
                    *self.tree.get_mut(&id).translation = translation;
                }

                self.tree.get_mut(&id).size = size;
            }

            visited_nodes.insert(id);

            // Schedule relayout if size changed.
            let node = self.tree.get(&id);
            if node.size != initial_size
                && let Some(parent_id) = node.parent
            {
                let parent_node = self.tree.get(&parent_id);
                let depth_node =
                    DepthNode::new(parent_node.depth, parent_id);

                self.scheduled_relayout.insert(depth_node);
                mutated_translations.insert(depth_node);
            }
        }

        // Propagate translations.
        for DepthNode { id, .. } in mutated_translations.into_iter() {
            let Some(node) = self.tree.nodes.get(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            // Translation could have already been resolved by a
            // previous iteration.
            if !node.translation.mutated() {
                continue;
            }

            self.propagate_translation(id);
        }
    }

    /// Propagrate the world translation from a given [`NodeId`].
    fn propagate_translation(&mut self, id: NodeId) {
        let mut node_stack = vec![(id, 0)];
        let mut translation_stack = vec![Vec2::ZERO];

        while let Some((id, index)) = node_stack.pop() {
            let Some(node) = self.tree.nodes.get_mut(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            node.world_translation =
                *node.translation + translation_stack[index];

            // Reset the mutation state once the world translation
            // is being updated.
            node.translation.reset_mutation();

            let new_index = translation_stack.len();
            translation_stack.push(node.world_translation);

            for child in node.children.iter() {
                node_stack.push((*child, new_index));
            }
        }
    }
}

/// [`NodeId`] cache with depth as the primary value for sorting.
#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
struct DepthNode {
    depth: u32,
    id: NodeId,
}

impl DepthNode {
    fn new(depth: u32, id: NodeId) -> Self {
        Self { depth, id }
    }
}

impl Rectree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a node into the tree while keeping track of the
    /// parent-child relationship.
    ///
    /// # Panics
    ///
    /// Panics if an invalid parent `NodeId` is used.
    pub fn insert(&mut self, mut node: RectNode) -> NodeId {
        let key = self.nodes.insert_with_key(|nodes, key| {
            let id = NodeId(key);
            if let Some(parent) = node.parent {
                let parent_node = nodes
                    .get_mut(&parent)
                    .expect("Invalid parent Id.");

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

    /// Removes a node and its children recursively.
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

    // TODO: RectreeFragment (above).
    // TODO: Support detach node -> fragment.
    // TODO: Support insert fragment.

    fn remove_recursive(&mut self, id: &NodeId) {
        let mut node_stack = vec![*id];

        while let Some(id) = node_stack.pop() {
            let Some(node) = self.nodes.get_mut(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            node_stack.extend(node.children());
            self.nodes.remove(&id);
        }
    }

    pub fn try_get(&self, id: &NodeId) -> Option<&RectNode> {
        self.nodes.get(id)
    }

    pub fn try_get_mut(
        &mut self,
        id: &NodeId,
    ) -> Option<&mut RectNode> {
        self.nodes.get_mut(id)
    }

    pub fn get(&self, id: &NodeId) -> &RectNode {
        self.try_get(id).unwrap_or_else(|| {
            panic!("{id} does not exists in tree.")
        })
    }

    pub fn get_mut(&mut self, id: &NodeId) -> &mut RectNode {
        self.try_get_mut(id).unwrap_or_else(|| {
            panic!("{id} does not exists in tree.")
        })
    }

    pub fn root_ids(&self) -> &HashSet<NodeId> {
        &self.root_ids
    }
}

pub trait Layouter {
    fn constraint(&self, id: &NodeId, tree: &Rectree) -> Constraint;

    fn build<F>(
        &self,
        id: &NodeId,
        tree: &Rectree,
        set_translation: F,
    ) -> Size
    where
        F: FnMut(NodeId, Vec2);
}

// TODO: Document that `None` means that it's flexible.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub width: Option<f64>,
    pub height: Option<f64>,
}

impl Constraint {
    pub fn from_both(width: f64, height: f64) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    pub fn from_width(width: f64) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    pub fn from_height(height: f64) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }

    pub fn from_none() -> Self {
        Self::default()
    }
}

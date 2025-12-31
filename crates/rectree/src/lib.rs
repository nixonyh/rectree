#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

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

    mutated_translations: BTreeSet<DepthNode>,
}

// TODO: Separate states from the tree data into "contexts".
pub struct LayoutCtx<'a> {
    tree: &'a mut Rectree,
    scheduled_relayout: BTreeSet<DepthNode>, // Should be moved from `EditCtx`.
    visited_nodes: HashSet<NodeId>,
    /// (node, constraint index) stack that is pending for
    /// [`Layouter::build()`].
    rebuild_stack: Vec<(NodeId, usize)>,
    child_stack: Vec<NodeId>,
    constraint_stack: Vec<Constraint>,
    new_translations: Vec<(NodeId, Vec2)>,
}

impl<'a> LayoutCtx<'a> {
    pub fn new(tree: &'a mut Rectree) -> Self {
        Self {
            tree,
            scheduled_relayout: BTreeSet::new(),
            visited_nodes: HashSet::new(),
            rebuild_stack: Vec::new(),
            child_stack: Vec::new(),
            constraint_stack: Vec::new(),
            new_translations: Vec::new(),
        }
    }

    pub fn schedule_relayout(&mut self, id: NodeId) -> bool {
        if let Some(node) = self.tree.get_node(&id) {
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
        self.visited_nodes.clear();

        // Pop the deepest nodes first to ensure children are finalized before parents.
        while let Some(depth_node) =
            self.scheduled_relayout.pop_last()
        {
            let id = depth_node.id;

            self.rebuild_stack.clear();
            self.child_stack.clear();
            self.constraint_stack.clear();

            let Some(node) = self.tree.get_node(&id) else {
                continue;
            };

            let initial_size = node.size;

            self.rebuild_stack.push((id, 0));
            self.child_stack.extend(node.children());
            self.constraint_stack.push(node.constraint);

            // Traverse the tree and create the build stack.
            while let Some(id) = self.child_stack.pop() {
                // Skip visited nodes, visited nodes should not be rebuilt.
                if self.visited_nodes.contains(&id) {
                    continue;
                }

                if let Some(node) = self.tree.get_node(&id) {
                    let constraint =
                        layouter.constraint(&id, self.tree);
                    let constraint_index =
                        self.constraint_stack.len();
                    self.constraint_stack.push(constraint);

                    // Nothing to rebuild if the constraint is still the same.
                    if node.constraint != constraint {
                        self.rebuild_stack
                            .push((id, constraint_index));
                    }
                    self.child_stack.extend(node.children());
                }
            }

            // Build out the size and position the children.
            for (id, constraint_index) in
                self.rebuild_stack.drain(..).rev()
            {
                self.new_translations.clear();

                let set_translation =
                    |id: NodeId, translation: Vec2| {
                        self.new_translations.push((id, translation));
                    };

                self.tree.with_node_mut(&id, |node| {
                    node.constraint =
                        self.constraint_stack[constraint_index];
                });

                // Build size.
                let size =
                    layouter.build(&id, self.tree, set_translation);

                // Update translation.
                for (id, translation) in
                    self.new_translations.drain(..)
                {
                    self.tree.with_node_mut(&id, |node| {
                        *node.translation = translation;
                    });
                }

                self.tree.with_node_mut(&id, |node| {
                    node.size = size;
                });
            }

            self.visited_nodes.insert(id);

            // Schedule relayout if size changed.
            if let Some(node) = self.tree.get_node(&id)
                && node.size != initial_size
                && let Some(parent_id) = node.parent
                && let Some(parent_node) =
                    self.tree.get_node(&parent_id)
            {
                self.scheduled_relayout.insert(DepthNode::new(
                    parent_node.depth,
                    parent_id,
                ));
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
    pub fn insert_node(&mut self, mut node: RectNode) -> NodeId {
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

            let mutated_node = DepthNode::new(node.depth, id);
            self.mutated_translations.insert(mutated_node);

            node
        });

        NodeId(key)
    }

    /// Removes a node and its children recursively.
    pub fn remove_node(&mut self, id: &NodeId) -> bool {
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

    pub fn get_node(&self, id: &NodeId) -> Option<&RectNode> {
        self.nodes.get(id)
    }

    pub fn with_node_mut<F, R>(
        &mut self,
        id: &NodeId,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut RectNode) -> R,
    {
        self.nodes.get_mut(id).map(|node| {
            let result = f(node);

            // Record changes.
            if node.translation.mutated() {
                self.mutated_translations
                    .insert(DepthNode::new(node.depth, *id));
            }

            result
        })
    }

    pub fn root_ids(&self) -> &HashSet<NodeId> {
        &self.root_ids
    }

    pub fn update_translations(&mut self) {
        let mutated_nodes =
            core::mem::take(&mut self.mutated_translations);

        for DepthNode { id, .. } in mutated_nodes.into_iter() {
            let Some(node) = self.nodes.get(&id) else {
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
            let Some(node) = self.nodes.get_mut(&id) else {
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

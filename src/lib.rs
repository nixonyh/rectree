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
pub struct NodeId(pub Key);

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
    scheduled_relayout: Vec<NodeId>, // Should be moved from `EditCtx`.
    node_stack: Vec<(NodeId, usize)>,
    node_child_stack: Vec<NodeId>,
    constraint_stack: Vec<Constraint>,
}

impl LayoutCtx<'_> {
    pub fn layout(&mut self, layouter: &impl Layouter) {
        while let Some(id) = self.scheduled_relayout.pop() {
            self.node_stack.clear();
            self.node_child_stack.clear();
            self.constraint_stack.clear();

            let Some(node) = self.tree.get_node(&id) else {
                continue;
            };

            self.node_stack.push((id, 0));
            self.node_child_stack.extend(node.children());
            self.constraint_stack.push(node.constraint());

            while let Some(id) = self.node_child_stack.pop()
                && let Some(node) = self.tree.get_node(&id)
            {
                if node.children().is_empty() {
                    continue;
                }

                let constraint = layouter.constraint(id);
                let constraint_index = self.constraint_stack.len();
                self.constraint_stack.push(constraint);

                for child in node.children() {
                    let Some(node) = self.tree.get_node(child) else {
                        continue;
                    };

                    // Nothing to rebuild if the constraint is still the same.
                    if node.constraint() != constraint {
                        self.node_stack
                            .push((*child, constraint_index));
                        self.node_child_stack.push(*child);
                    }
                }
            }

            for (id, index) in self.node_stack.drain(..).rev() {
                let constraint = self.constraint_stack[index];
                let size = layouter.build(id, constraint, self.tree);

                let Some(node) = self.tree.get_node_mut(&id) else {
                    continue;
                };

                // TODO: Reset size mutation state?
                // Do we need mut_detect here, we could just cache it..?
                // node.size.set_if_ne(size);
                *node.size = size;
                *node.constraint = constraint;
            }
        }
    }
}

pub struct EditCtx<'a> {
    tree: &'a mut Rectree,
    scheduled_relayout: BTreeSet<DepthNode>,
}

impl<'a> EditCtx<'a> {
    pub fn schedule_relayout(&mut self, id: NodeId) -> bool {
        if let Some(node) = self.tree.get_node(&id) {
            return self
                .scheduled_relayout
                .insert(DepthNode::new(node.depth, id));
        }

        false
    }

    // TODO: Think of a better name.
    pub fn compile(self) -> LayoutCtx<'a> {
        LayoutCtx {
            tree: self.tree,
            scheduled_relayout: self
                .scheduled_relayout
                .into_iter()
                // Layout happens bottom-up.
                .rev()
                .map(|n| n.id)
                .collect(),
            node_stack: Vec::new(),
            node_child_stack: Vec::new(),
            constraint_stack: Vec::new(),
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

    pub fn get_node_mut(
        &mut self,
        id: &NodeId,
    ) -> Option<&mut RectNode> {
        self.nodes.get_mut(id)
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
            // if node.size.mutated() {
            //     let depth_node = DepthNode::new(node.depth, *id);
            // }
            if node.local_translation.mutated() {
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
            if !node.local_translation.mutated() {
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
                *node.local_translation + translation_stack[index];

            // Reset the mutation state once the world translation
            // is being updated.
            node.local_translation.reset_mutation();

            let new_index = translation_stack.len();
            translation_stack.push(node.world_translation);

            for child in node.children.iter() {
                node_stack.push((*child, new_index));
            }
        }
    }
}

pub trait Layouter {
    fn constraint(&self, id: NodeId) -> Constraint;

    fn build(
        &self,
        id: NodeId,
        constraint: Constraint,
        tree: &mut Rectree,
    ) -> Size;

    // fn build_and_reset(
    //     &self,
    //     id: NodeId,
    //     constraint: Constraint,
    //     tree: &mut Rectree,
    // ) {
    //     self.build(id, constraint, tree);
    //     if let Some(node) = tree.get_node(&id) {
    //         for child in node
    //             .children()
    //             .iter()
    //             .copied()
    //             .collect::<alloc::boxed::Box<_>>()
    //         {
    //             if let Some(node) = tree.get_node_mut(&child) {
    //                 node.size.reset_mutation();
    //             }
    //         }
    //     }
    // }
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

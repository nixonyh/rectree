use alloc::collections::btree_set::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;

use crate::node::RectNode;
use crate::{NodeId, Rectree};

/// A 2D size in resolved pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self::splat(0.0);

    pub const INFINITY: Self = Self::splat(f32::INFINITY);

    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }
}

/// A 2D position or translation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self::splat(0.0);

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }
}

impl core::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

/// A min/max size constraint passed down the element tree.
///
/// `max` fields set to [`f32::INFINITY`] indicate an unconstrained
/// axis. Use the constructor helpers [`Self::tight()`],
/// [`Self::loose()`], [`Self::unbounded()`] rather than constructing
/// directly where possible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub min: Size,
    pub max: Size,
}

impl Constraint {
    /// Forces the child to be exactly `size`.
    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    /// Child may choose any size from zero up to `max`.
    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::ZERO,
            max,
        }
    }

    /// No bounds on either axis.
    pub const fn unbounded() -> Self {
        Self {
            min: Size::ZERO,
            max: Size::INFINITY,
        }
    }

    /// Bounded width, unbounded height
    /// (e.g. vertical scroll container).
    pub const fn fixed_width(width: f32) -> Self {
        Self {
            min: Size::ZERO,
            max: Size {
                width,
                height: f32::INFINITY,
            },
        }
    }

    /// Bounded height, unbounded width
    /// (e.g. horizontal scroll container).
    pub const fn fixed_height(height: f32) -> Self {
        Self {
            min: Size::ZERO,
            max: Size {
                width: f32::INFINITY,
                height,
            },
        }
    }

    /// Clamps `size` so it satisfies this constraint.
    pub const fn constrain(&self, size: Size) -> Size {
        Size {
            width: size.width.max(self.min.width).min(self.max.width),
            height: size
                .height
                .max(self.min.height)
                .min(self.max.height),
        }
    }
}

impl Default for Constraint {
    fn default() -> Self {
        Self::unbounded()
    }
}

/// Callback interface for reading and writing child layout state
/// during [`crate::layout::LayoutWorld::build`].
pub trait Layouter {
    type Id;

    fn get_size(&self, id: &Self::Id) -> Size;

    fn set_position(&mut self, id: &Self::Id, position: Vec2);
}

/// Layout execution.
impl Rectree {
    /// Check if we need to call [`Self::layout()`].
    pub fn needs_relayout(&self) -> bool {
        !self.scheduled_relayout.is_empty()
    }

    /// Schedules a node for relayout.
    ///
    /// Returns `true` if the node was newly scheduled, or `false`
    /// if the node does not exist or was already scheduled.
    pub fn schedule_relayout(&mut self, id: NodeId) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.state.reset();
            return self
                .scheduled_relayout
                .insert(DepthNode::new(node.depth, id));
        }

        false
    }

    /// Executes the layout pass using the provided [`LayoutWorld`].
    pub fn layout<W>(&mut self, world: &W)
    where
        W: LayoutWorld,
    {
        let scheduled_relayout =
            core::mem::take(&mut self.scheduled_relayout);
        let mut child_stack = Vec::<NodeId>::new();
        let mut build_stack = BTreeSet::<DepthNode>::new();

        for DepthNode { id, .. } in scheduled_relayout.iter() {
            let Some(node) = self.try_get_mut(id) else {
                continue;
            };
            // Check constrain flag, if it has already been
            // constrained, skip the entire process.
            if node.state.constrained() {
                continue;
            }

            child_stack.push(*id);

            // Recursively propagate constraint from parent to child.
            while let Some(id) = child_stack.pop() {
                let parent_constraint =
                    self.get(&id).parent_constraint;
                let constraint =
                    world.constraint(&id, parent_constraint);

                self.nodes.scope(&id, |nodes, node| {
                    node.state.has_recontrained();

                    for child in node.children() {
                        let child_node =
                            Self::get_node_mut(nodes, child);

                        // Skip if constraint is still the same.
                        if child_node.parent_constraint != constraint
                        {
                            child_node.parent_constraint = constraint;
                            child_stack.push(*child);
                        }
                    }
                });

                let node = self.get_mut(&id);
                node.state.needs_rebuild();
                build_stack.insert(DepthNode::new(node.depth, id));
            }
        }

        let mut positioner = Positioner::default();
        let mut translation_stack = scheduled_relayout;

        // Propagate size from child to parent.
        while let Some(DepthNode { id, .. }) = build_stack.pop_last()
        {
            let size = world.build(
                &id,
                self.get(&id),
                self,
                &mut positioner,
            );
            positioner.apply(self);

            self.nodes.scope(&id, |nodes, node| {
                node.state.has_rebuilt();
                // Parent needs to be rebuilt if size changes.
                if node.size != size {
                    if let Some(parent) = node.parent {
                        let parent_node =
                            Self::get_node_mut(nodes, &parent);
                        // Insert only if parent node is not already
                        // set to be rebuilt.
                        if parent_node.state.built() {
                            parent_node.state.needs_reposition();
                            parent_node.state.needs_rebuild();

                            let depth_node = DepthNode::new(
                                parent_node.depth,
                                parent,
                            );
                            translation_stack.insert(depth_node);
                            build_stack.insert(depth_node);
                        }
                    }
                    node.size = size;
                }
            });
        }

        // Propagate translations from parent to child.
        for DepthNode { id, .. } in translation_stack.into_iter() {
            let node = self.get(&id);

            // Translation could have already been resolved by a
            // previous iteration.
            if node.state.positioned() {
                continue;
            }

            self.propagate_translation(id);
        }
    }

    /// Propagates world-space translations starting from a node.
    ///
    /// This updates the node's world translation and recursively
    /// applies it to all descendants, clearing translation mutation
    /// flags in the process.
    fn propagate_translation(&mut self, id: NodeId) {
        let mut node_stack = vec![(id, 0)];
        let mut translation_stack = vec![Vec2::ZERO];

        while let Some((id, index)) = node_stack.pop() {
            let node = self.get_mut(&id);

            node.world_translation =
                node.translation + translation_stack[index];

            // This node is now positioned since the world
            // translation has been updated.
            node.state.has_repositioned();

            let new_index = translation_stack.len();
            translation_stack.push(node.world_translation);

            for child in node.children.iter() {
                node_stack.push((*child, new_index));
            }
        }
    }
}

/// Provides the layout logic for each node in the tree.
///
/// Acts as the bridge between [`Rectree`] and the application's
/// element system.
pub trait LayoutWorld {
    /// Computes the constraint this node propagates to its children.
    ///
    /// `parent` is the constraint imposed on this node by its own
    /// parent. The return value is applied to each child before
    /// their build pass.
    fn constraint(
        &self,
        id: &NodeId,
        parent: Constraint,
    ) -> Constraint;

    /// Builds the layout for a node and returns its resolved size.
    ///
    /// Called bottom-up after constraints have been propagated.
    /// Implementations may inspect the tree and assign child
    /// translations via [`Positioner`].
    fn build(
        &self,
        id: &NodeId,
        node: &RectNode,
        tree: &Rectree,
        pos: &mut Positioner,
    ) -> Size;
}

/// Collects child translations produced during layout construction.
///
/// See [`LayoutWorld::build()`].
#[derive(Default)]
pub struct Positioner {
    new_translations: Vec<(NodeId, Vec2)>,
}

impl Positioner {
    /// Sets the local translation for a node.
    ///
    /// The translation is recorded and applied later as part of the
    /// layout commit phase. If multiple translations are set for the
    /// same node, the last one wins.
    pub fn set(&mut self, id: NodeId, translation: Vec2) {
        self.new_translations.push((id, translation));
    }

    /// Applies all recorded translations to the [`Rectree`].
    ///
    /// This is called internally after layout resolution to commit
    /// the results of [`LayoutWorld::build()`].
    fn apply(&mut self, tree: &mut Rectree) {
        for (id, translation) in self.new_translations.drain(..) {
            tree.get_mut(&id).translation = translation;
        }
    }
}

/// [`NodeId`] cache with depth as the primary value for sorting.
#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct DepthNode {
    depth: u32,
    id: NodeId,
}

impl DepthNode {
    pub fn new(depth: u32, id: NodeId) -> Self {
        Self { depth, id }
    }
}

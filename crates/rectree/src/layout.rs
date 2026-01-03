use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashSet;
use kurbo::{Size, Vec2};

use crate::{NodeId, Rectree};

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
        if let Some(node) = self.try_get(&id) {
            return self
                .scheduled_relayout
                .insert(DepthNode::new(node.depth, id));
        }

        false
    }

    /// Executes the layout pass using the provided [`LayoutSolver`].
    ///
    /// This method performs an incremental, bottom-up layout:
    /// - Constraints are recomputed as needed.
    /// - Sizes are rebuilt starting from the deepest affected nodes.
    /// - Translations are updated and propagated through the tree.
    ///
    /// After completion, all scheduled nodes and their affected
    /// ancestors will have up-to-date size and world translation.
    pub fn layout<T>(&mut self, solver: &T)
    where
        T: LayoutSolver,
    {
        // Initialize reusable heap allocations.
        let mut scheduled_relayout =
            core::mem::take(&mut self.scheduled_relayout);
        let mut visited_nodes = HashSet::<NodeId>::new(); // TODO: Could we avoid using a hashset?
        // (node, constraint index) stack pending for layout build.
        let mut rebuild_stack = Vec::<(NodeId, usize)>::new();
        let mut child_stack = Vec::<NodeId>::new();
        let mut constraint_stack = Vec::<Constraint>::new();
        let mut positioner = Positioner::default();
        let mut mutated_translations = scheduled_relayout.clone();

        // Pop the deepest nodes first to ensure children are
        // finalized before parents.
        while let Some(DepthNode { id, .. }) =
            scheduled_relayout.pop_last()
        {
            rebuild_stack.clear();
            child_stack.clear();
            constraint_stack.clear();

            let node = self.get(&id);
            let initial_size = node.size;

            rebuild_stack.push((id, 0));
            child_stack.extend(node.children());
            // TODO: Assign constraint directly to the node?
            let constraint = node
                .parent
                .map(|id| solver.constraint(&id))
                .unwrap_or_default();
            constraint_stack.push(constraint);

            // Traverse the tree and create the build stack.
            while let Some(id) = child_stack.pop() {
                // Skip visited nodes.
                if visited_nodes.contains(&id) {
                    continue;
                }

                let node = self.get(&id);
                let constraint = solver.constraint(&id);
                let constraint_index = constraint_stack.len();
                constraint_stack.push(constraint);

                for child in node.children() {
                    // Nothing to rebuild if the constraint is still
                    // the same.
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
                // TODO: Could this be assigned above? If so, we can
                // remove the constraint stack.
                self.get_mut(&id).constraint =
                    constraint_stack[constraint_index];

                // Build size.
                let size = solver.build(&id, self, &mut positioner);

                // Update translation.
                positioner.apply(self);
                // Update size.
                self.get_mut(&id).size = size;
            }

            visited_nodes.insert(id);

            // Schedule relayout if size changed.
            let node = self.get(&id);
            if node.size != initial_size
                && let Some(parent) = node.parent
            {
                let parent_node = self.get(&parent);
                let depth_node =
                    DepthNode::new(parent_node.depth, parent);

                scheduled_relayout.insert(depth_node);
                mutated_translations.insert(depth_node);
            }
        }

        // Propagate translations.
        for DepthNode { id, .. } in mutated_translations.into_iter() {
            let node = self.get(&id);

            // Translation could have already been resolved by a
            // previous iteration.
            if !node.translation.mutated() {
                continue;
            }

            self.propagate_translation(id);
        }
    }

    /// Propagates world-space translations starting from a node.
    ///
    /// This updates the node’s world translation and recursively
    /// applies it to all descendants, clearing translation mutation
    /// flags in the process.
    fn propagate_translation(&mut self, id: NodeId) {
        let mut node_stack = vec![(id, 0)];
        let mut translation_stack = vec![Vec2::ZERO];

        while let Some((id, index)) = node_stack.pop() {
            let node = self.get_mut(&id);

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

/// Defines a layout algorithm for [`Rectree`].
pub trait LayoutSolver {
    /// Computes the layout constraint for a node.
    ///
    /// Constraint must be known without any context of the tree.
    fn constraint(&self, id: &NodeId) -> Constraint;

    /// Builds the layout for a node and returns its final size.
    ///
    /// Implementations may assign translations to child nodes via
    /// [`Positioner`]. All translations are relative to their parent.
    fn build(
        &self,
        id: &NodeId,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size;
}

#[derive(Default)]
pub struct Positioner {
    new_translations: Vec<(NodeId, Vec2)>,
}

impl Positioner {
    pub fn set(&mut self, id: NodeId, translation: Vec2) {
        self.new_translations.push((id, translation));
    }

    pub fn apply(&mut self, tree: &mut Rectree) {
        for (id, translation) in self.new_translations.drain(..) {
            *tree.get_mut(&id).translation = translation;
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

/// Size constraints applied to a node during layout.
///
/// A value of `Some(f64)` fixes the corresponding dimension to an
/// explicit size, while `None` indicates that the dimension is
/// unconstrained (flexible) and may be determined by layout.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    // Fixed width constraint, or `None` if flexible.
    pub width: Option<f64>,
    // Fixed height constraint, or `None` if flexible.
    pub height: Option<f64>,
}

impl Constraint {
    /// Creates a constraint with both width and height fixed.
    pub fn fixed(width: f64, height: f64) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    /// Creates a constraint with a fixed width and flexible height.
    pub fn fixed_width(width: f64) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    /// Creates a constraint with a fixed height and flexible width.
    pub fn fixed_height(height: f64) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }

    /// Creates a fully flexible constraint with no fixed dimensions.
    pub fn flexible() -> Self {
        Self::default()
    }
}

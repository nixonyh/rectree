use bitflags::bitflags;

use crate::geom::{Constraint, Size, Vec2};

/// Per-node layout data stored in [`crate::RectNodes`].
///
/// A `RectNode` holds all the numbers that rectree reads and
/// writes during a layout cycle. It carries no widget logic;
/// that lives in your [`crate::Rectree`] implementation.
pub struct RectNode<Id> {
    /// The parent node's ID, or `None` for root nodes.
    ///
    /// Used by the rebuild pass to walk upward when a child's
    /// size changes.
    pub parent_id: Option<Id>,

    /// Flags tracking which layout passes are current for this
    /// node.
    ///
    /// Check and clear these flags to control incremental
    /// re-layout.
    pub state: NodeState,

    /// The constraint imposed on this node by its parent.
    ///
    /// Written by [`crate::constrain`]; read by [`crate::build`].
    pub constraint: Constraint,

    /// The resolved size of this node.
    ///
    /// Written by [`crate::build`]; read by parent nodes during
    /// their own build step and by
    /// [`crate::propagate_translation`].
    pub size: Size,

    /// Local translation relative to the parent node's origin.
    ///
    /// Written by the parent's build step via
    /// `RectContext::set_translation`. Zero by default.
    pub translation: Vec2,

    /// Absolute world-space position of this node's origin.
    ///
    /// Written by [`crate::propagate_translation`] as the sum of
    /// all ancestor translations. Use this value for rendering.
    pub world_translation: Vec2,
}

impl<Id> RectNode<Id> {
    /// Creates a new node with the given parent and all other
    /// fields at their defaults (`Constraint::unbounded()`,
    /// `Size::ZERO`, `Vec2::ZERO`, empty [`NodeState`]).
    pub fn new(parent_id: Option<Id>) -> Self {
        Self {
            parent_id,
            state: NodeState::default(),
            constraint: Constraint::default(),
            size: Size::default(),
            translation: Vec2::default(),
            world_translation: Vec2::default(),
        }
    }
}

bitflags! {
    /// Tracks which layout passes have completed for a [`RectNode`].
    ///
    /// Each pass sets its flag when it finishes processing a node.
    /// Before processing, a pass checks the flag and skips the
    /// node (and its subtree) if it is already set and the
    /// relevant input is unchanged.
    ///
    /// Clearing a flag (via `needs_*`) schedules the node for
    /// reprocessing on the next layout call.
    ///
    /// # Flag lifecycle
    ///
    /// | Event                          | Flags changed                      |
    /// | ------------------------------ | ---------------------------------- |
    /// | Node created or [`Self::reset`]| all cleared                        |
    /// | Constraint passes through      | `CONSTRAINED` set                  |
    /// | Constraint changes             | `CONSTRAINED` set, `BUILT` cleared |
    /// | Build completes                | `BUILT` set, `POSITIONED` cleared  |
    /// | Translation propagated         | `POSITIONED` set                   |
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NodeState: u8 {
        /// Set by [`crate::propagate_translation`] when
        /// `world_translation` is current.
        const POSITIONED  = 1;

        /// Set by [`crate::constrain`] when the stored constraint
        /// matches the value last propagated from the parent.
        const CONSTRAINED = 1 << 1;

        /// Set by [`crate::build`] or [`crate::build_up`] when
        /// `size` and child translations are current.
        const BUILT       = 1 << 2;
    }
}

impl NodeState {
    /// Clears all flags, scheduling the node for a full
    /// re-layout.
    pub fn reset(&mut self) {
        *self = Self::empty();
    }

    /// Returns `true` when all three passes are current.
    ///
    /// Used by the `layout` free function to short-circuit the
    /// entire cycle.
    pub fn is_ready(&self) -> bool {
        *self == Self::all()
    }

    /// Returns `true` if the `POSITIONED` flag is set.
    pub fn is_positioned(&self) -> bool {
        self.intersects(Self::POSITIONED)
    }

    /// Returns `true` if the `CONSTRAINED` flag is set.
    pub fn is_constrained(&self) -> bool {
        self.intersects(Self::CONSTRAINED)
    }

    /// Returns `true` if the `BUILT` flag is set.
    pub fn is_built(&self) -> bool {
        self.intersects(Self::BUILT)
    }

    /// Clears `POSITIONED`, marking translation as stale.
    pub fn needs_reposition(&mut self) {
        self.remove(Self::POSITIONED);
    }

    /// Clears `CONSTRAINED`, marking constraint as stale.
    pub fn needs_reconstrain(&mut self) {
        self.remove(Self::CONSTRAINED);
    }

    /// Clears `BUILT`, marking size and translations as stale.
    pub fn needs_rebuild(&mut self) {
        self.remove(Self::BUILT);
    }

    /// Sets `POSITIONED`, marking translation as current.
    pub fn has_repositioned(&mut self) {
        self.insert(Self::POSITIONED);
    }

    /// Sets `CONSTRAINED`, marking constraint as current.
    pub fn has_reconstrained(&mut self) {
        self.insert(Self::CONSTRAINED);
    }

    /// Sets `BUILT`, marking size and translations as current.
    pub fn has_rebuilt(&mut self) {
        self.insert(Self::BUILT);
    }
}

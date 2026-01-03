use hashbrown::HashSet;
use kurbo::{Rect, Size, Vec2};

use crate::NodeId;
use crate::layout::Constraint;
use crate::mut_detect::MutDetect;

/// An axis-aligned rectangle in the layout tree.
///
/// The rectangle is defined in **local space** by a translation and
/// a size. `local_translation` denotes the **top-left corner**
/// relative to the parent. The final position in world space is
/// stored in `world_translation` after layout resolution.
///
/// ```text
/// translation
/// ^
/// +--------+
/// |        | height
/// +--------+
///   width
/// ```
#[derive(Default, Debug, Clone)]
pub struct RectNode {
    /// Local translation, relative to the parent.
    pub(crate) translation: MutDetect<Vec2>,
    /// Size of the node.
    ///
    /// This is the resolved size after
    /// [`crate::layout::LayoutSolver::build()`].
    pub(crate) size: Size,
    /// Constraint imposed by the parent onto this node.
    ///
    /// This is computed during the top-down constraint pass and should
    /// not be mutated directly by user code.
    pub(crate) parent_constraint: Constraint,
    // /// Constraint of this node to be given to its children.
    // pub(crate) constraint: Constraint,
    /// World-space translation of this node.
    ///
    /// This is the accumulated translation from the root and is
    /// computed during transform propagation.
    pub(crate) world_translation: Vec2,
    /// Parent node in the hierarchy, if any.
    pub(crate) parent: Option<NodeId>,
    /// Child nodes of this node.
    pub(crate) children: HashSet<NodeId>,
    /// How deep in the hierarchy is this node (0 for root nodes).
    ///
    /// This value is assigned and maintained by [`crate::Rectree`]
    /// and must not be modified externally.
    pub(crate) depth: u32,
    /// Set to false when this node needs to be reconstrained.
    pub(crate) constrained: bool,
    /// Set to false when this nodes needs to be rebuilt.
    pub(crate) built: bool,
}

/// Builders.
impl RectNode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_translation(translation: impl Into<Vec2>) -> Self {
        Self::new().with_translation(translation)
    }

    pub fn from_size(size: impl Into<Size>) -> Self {
        Self::new().with_size(size)
    }

    pub fn from_translation_size(
        translation: impl Into<Vec2>,
        size: impl Into<Size>,
    ) -> Self {
        Self::new().with_translation(translation).with_size(size)
    }

    pub fn from_rect(rect: impl Into<Rect>) -> Self {
        let rect: Rect = rect.into();
        Self::new()
            .with_translation(Vec2::new(rect.min_x(), rect.min_y()))
            .with_size(rect.size())
    }

    pub fn with_translation(
        mut self,
        translation: impl Into<Vec2>,
    ) -> Self {
        *self.translation = translation.into();
        self
    }

    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }
}

/// Getters.
impl RectNode {
    pub fn translation(&self) -> Vec2 {
        *self.translation
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn parent_constraint(&self) -> Constraint {
        self.parent_constraint
    }

    pub fn world_translation(&self) -> Vec2 {
        self.world_translation
    }

    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn children(&self) -> &HashSet<NodeId> {
        &self.children
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn world_rect(&self) -> Rect {
        Rect::new(
            self.world_translation.x,
            self.world_translation.y,
            self.world_translation.x + self.size.width,
            self.world_translation.y + self.size.height,
        )
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

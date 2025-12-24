use hashbrown::HashSet;
use kurbo::{Rect, Size, Vec2};

use crate::mut_detect::MutDetect;
use crate::{Constraint, NodeId};

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
    pub local_translation: MutDetect<Vec2>,
    pub size: MutDetect<Size>,
    /// Constraint from the parent.
    pub(crate) constraint: MutDetect<Constraint>,
    pub(crate) world_translation: Vec2,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: MutDetect<HashSet<NodeId>>,
    /// How deep in the hierarchy is this node (0 for root nodes).
    pub(crate) depth: u32,
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
        *self.local_translation = translation.into();
        self
    }

    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        *self.size = size.into();
        self
    }

    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }
}

/// Getters.
impl RectNode {
    pub fn constraint(&self) -> Constraint {
        *self.constraint
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
}

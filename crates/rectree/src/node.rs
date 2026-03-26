use bitflags::bitflags;
use hashbrown::HashSet;
use kurbo::{Rect, Size, Vec2};

use crate::NodeId;
use crate::layout::Constraint;

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
    /// See [`Self::translation()`].
    pub(crate) translation: Vec2,
    /// See [`Self::size()`].
    pub(crate) size: Size,
    /// See [`Self::parent_constraint()`].
    pub(crate) parent_constraint: Constraint,
    /// See [`Self::world_translation()`].
    pub(crate) world_translation: Vec2,
    /// See [`Self::parent()`].
    pub(crate) parent: Option<NodeId>,
    /// See [`Self::children()`].
    pub(crate) children: HashSet<NodeId>,
    /// See [`Self::depth()`].
    pub(crate) depth: u32,
    /// The state of the current node.
    pub(crate) state: NodeState,
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
        self.translation = translation.into();
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
    /// Local translation, relative to the parent.
    pub fn translation(&self) -> Vec2 {
        self.translation
    }

    /// Size of the node.
    ///
    /// This is the resolved size after
    /// [`crate::layout::LayoutWorld::build()`].
    pub fn size(&self) -> Size {
        self.size
    }

    /// Constraint imposed by the parent onto this node.
    ///
    /// This is computed during the top-down constraint pass via
    /// [`crate::layout::LayoutWorld::constraint()`].
    pub fn parent_constraint(&self) -> Constraint {
        self.parent_constraint
    }

    /// World-space translation of this node.
    ///
    /// This is the accumulated translation from the root and is
    /// computed during transform propagation.
    pub fn world_translation(&self) -> Vec2 {
        self.world_translation
    }

    /// Parent node in the hierarchy, if any.
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Child nodes of this node.
    pub fn children(&self) -> &HashSet<NodeId> {
        &self.children
    }

    /// How deep in the hierarchy is this node (0 for root nodes).
    ///
    /// This value is assigned and maintained by [`crate::Rectree`]
    /// and must not be modified externally.
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Compute the world space [`Rect`] from
    /// [`Self::world_translation`] and [`Self::size`].
    pub fn world_rect(&self) -> Rect {
        Rect::new(
            self.world_translation.x,
            self.world_translation.y,
            self.world_translation.x + self.size.width,
            self.world_translation.y + self.size.height,
        )
    }

    /// Returns `true` if [`Self::parent`] is `None`.
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct NodeState: u8 {
        const POSITIONED = 1;
        const CONSTRAINED = 1 << 1;
        const BUILT = 1 << 2;
    }
}

impl NodeState {
    /// Returns the [`Self::POSITIONED`] flag value.
    pub fn positioned(&self) -> bool {
        self.intersects(Self::POSITIONED)
    }

    /// Returns the [`Self::CONSTRAINED`] flag value.
    pub fn constrained(&self) -> bool {
        self.intersects(Self::CONSTRAINED)
    }

    /// Returns the [`Self::BUILT`] flag value.
    pub fn built(&self) -> bool {
        self.intersects(Self::BUILT)
    }

    pub fn reset(&mut self) {
        *self = Self::empty();
    }

    /// Set [`Self::POSITIONED`] flag to `false`.
    pub fn needs_reposition(&mut self) {
        self.remove(Self::POSITIONED);
    }

    /// Set [`Self::CONSTRAINED`] flag to `false`.
    pub fn needs_reconstrain(&mut self) {
        self.remove(Self::CONSTRAINED);
    }

    /// Set [`Self::BUILT`] flag to `false`.
    pub fn needs_rebuild(&mut self) {
        self.remove(Self::BUILT);
    }

    /// Set [`Self::POSITIONED`] flag to `true`.
    pub fn has_repositioned(&mut self) {
        self.insert(Self::POSITIONED);
    }

    /// Set [`Self::CONSTRAINED`] flag to `true`.
    pub fn has_recontrained(&mut self) {
        self.insert(Self::CONSTRAINED);
    }

    /// Set [`Self::BUILT`] flag to `true`.
    pub fn has_rebuilt(&mut self) {
        self.insert(Self::BUILT);
    }
}

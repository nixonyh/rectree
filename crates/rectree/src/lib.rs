#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub use geom::{Constraint, Size, Vec2};
pub use node::{NodeState, RectNode};

pub mod geom;
pub mod node;

/// Tree structure and per-node layout logic for a rectree hierarchy.
///
/// `Rectree` is the read-only half of the layout split. It defines
/// how nodes are connected ([`Self::for_each_child`]) and how each node
/// computes its constraint ([`Self::constrain`]) and size
/// ([`Self::build`]). The mutable per-node data lives separately in
/// [`RectNodes`].
pub trait Rectree {
    type Id;
    type Nodes: NodeContext<Id = Self::Id>;

    /// Calls `f` for each direct child of `id` in layout order.
    ///
    /// `nodes` is threaded through to the closure so that
    /// implementations that dispatch through per-node metadata
    /// (e.g. a type-erased tree) can read it without a separate
    /// borrow that would conflict with the `&mut Self::Nodes` held
    /// by the calling layout pass.
    fn for_each_child(
        &self,
        id: &Self::Id,
        nodes: &mut Self::Nodes,
        f: impl FnMut(&Self::Id, &mut Self::Nodes),
    );

    /// Derives the constraint this node passes to its children
    /// from the constraint `parent` imposed on this node.
    ///
    /// Most nodes return `parent` unchanged (pass-through). A
    /// padding container would subtract its insets; a fixed-size
    /// container would ignore `parent` and return a tight
    /// constraint.
    ///
    /// `nodes` is a shared view of the node storage, available for
    /// implementations that store per-node metadata (such as a
    /// type tag) inside the nodes map rather than in the tree.
    ///
    /// Called top-down by [`constrain`].
    fn constrain(
        &self,
        id: &Self::Id,
        nodes: &Self::Nodes,
        parent: Constraint,
    ) -> Constraint;

    /// Measures this node given `constraint` and the already-built
    /// children, returning the node's resolved [`Size`].
    ///
    /// Children are guaranteed to be fully built before this is
    /// called (bottom-up ordering). The implementation may:
    ///
    /// - Read child sizes via `nodes.get_size(child_id)`.
    /// - Write child local translations via
    ///   `nodes.set_translation(child_id, pos)`.
    ///
    /// It must not mutate child sizes. `nodes` is a [`NodeContext`]
    /// which intentionally limits access to reads and translation
    /// writes only.
    ///
    /// Called bottom-up by [`build`].
    fn build(
        &self,
        id: &Self::Id,
        constraint: Constraint,
        nodes: &mut Self::Nodes,
    ) -> Size;
}

/// Flat storage for [`RectNode`]s keyed by an application-defined
/// `Id`.
///
/// This is the mutable half of the layout split. It holds only
/// per-node numbers (`constraint`, `size`, `translation`) and
/// exposes them to the rectree free functions. It knows nothing
/// about tree structure or layout logic; those live in [`Rectree`].
///
/// Any type that implements `RectNodes` automatically implements
/// [`NodeContext`] through a blanket impl.
///
/// # Splitting storage from tree logic
///
/// rectree's free functions take `tree: &T` and `nodes: &mut N`
/// as two separate arguments. This lets Rust borrow `T` immutably
/// (for traversal and logic) and `N` mutably (for data writes) at
/// the same time, which would be impossible if a single type owned
/// both.
pub trait RectNodes {
    type Id;

    fn get_node(&self, id: &Self::Id) -> Option<&RectNode<Self::Id>>;

    fn get_node_mut(
        &mut self,
        id: &Self::Id,
    ) -> Option<&mut RectNode<Self::Id>>;
}

/// Blanket impl: any [`RectNodes`] storage is automatically a
/// [`NodeContext`].
///
/// This means you never implement `NodeContext` by hand. Just
/// implement `RectNodes` and the restricted build-time view
/// comes for free.
impl<N: RectNodes> NodeContext for N {
    type Id = N::Id;

    fn get_size(&self, id: &Self::Id) -> Size {
        self.get_node(id).map(|n| n.size).unwrap_or(Size::ZERO)
    }

    fn set_translation(&mut self, id: &Self::Id, translation: Vec2) {
        if let Some(n) = self.get_node_mut(id) {
            n.translation = translation;
        }
    }
}

/// Restricted view of [`RectNodes`] exposed to [`Rectree::build`].
///
/// During the build pass, a widget must be able to:
///
/// - Read the resolved [`Size`] of its children (`get_size`).
/// - Write local translations to position its children
///   (`set_translation`).
///
/// It must not mutate child sizes directly, because the build
/// pass processes nodes bottom-up and a size written here would
/// silently invalidate the ordering guarantee.
///
/// `NodeContext` is never implemented manually. Any type that
/// implements [`RectNodes`] gets `NodeContext` for free through
/// a blanket impl in `lib.rs`.
pub trait NodeContext {
    type Id;

    /// Returns the resolved size of the node identified by `id`.
    ///
    /// Returns [`Size::ZERO`] if the id is not found.
    fn get_size(&self, id: &Self::Id) -> Size;

    /// Sets the local translation of the node identified by `id`.
    ///
    /// This is the position relative to the parent's origin.
    /// [`propagate_translation`] later accumulates these
    /// into absolute world positions.
    fn set_translation(&mut self, id: &Self::Id, position: Vec2);
}

/// Runs a full layout cycle on the subtree rooted at `id`.
///
/// Executes the three passes in order:
///
/// 1. [`constrain`] (top-down): propagates constraints from
///    parent to children.
/// 2. [`build`] (bottom-up): measures nodes and writes child
///    translations.
/// 3. [`propagate_translation`] (top-down): accumulates local
///    translations into absolute `world_translation` values.
///
/// Each pass is short-circuited by [`NodeState`] flags so only
/// nodes that actually changed are reprocessed. To force a full
/// re-layout of the subtree, reset the root node's state before
/// calling:
///
/// ```rust,ignore
/// nodes.get_node_mut(&root_id).unwrap().state.reset();
/// layout(&tree, &mut nodes, &root_id);
/// ```
///
/// If the node's size changes and it has a parent, the parent
/// and ancestors are re-measured via an upward rebuild pass
/// before translation is propagated.
///
/// # Panics
///
/// Panics if `id` is not present in `nodes`.
pub fn layout<
    Id: Copy,
    T: Rectree<Id = Id, Nodes = N>,
    N: RectNodes<Id = Id>,
>(
    tree: &T,
    nodes: &mut N,
    id: &Id,
) {
    let node = nodes.get_node(id).expect("layout: Id is invalid!");

    let old_size = node.size;
    let parent = node.parent_id;

    if node.state.is_ready() {
        return;
    }

    // 1. Constrain down the hierarchy.
    constrain(tree, nodes, id, node.constraint);

    // 2. Build sizes up the hierarchy.
    build(tree, nodes, id);

    let new_size = nodes.get_size(id);

    // Size changed; propagate upward without re-traversing children.
    let mut bubbled_id = *id;
    if new_size != old_size
        && let Some(ref parent_id) = parent
    {
        bubbled_id = build_up(tree, nodes, parent_id);
    }

    // 3. Propagate translation, seeding from `bubbled_id`'s parent
    // world translation. Seeding from `bubbled_id`'s own translation
    // would offset a non-root node by its own translation each pass;
    // the root has no parent and seeds from zero.
    let parent_world = nodes
        .get_node(&bubbled_id)
        .and_then(|node| node.parent_id)
        .and_then(|parent| nodes.get_node(&parent))
        .map_or(Vec2::ZERO, |node| node.world_translation);
    propagate_translation(
        tree,
        nodes,
        &bubbled_id,
        parent_world,
        false,
    );
}

/// Propagates a constraint top-down through the subtree rooted
/// at `id`.
///
/// `parent` is the constraint imposed on this node by its
/// parent. It is stored on the node then narrowed via
/// [`Rectree::constrain`] to produce the constraint passed to
/// children.
///
/// # Short-circuit behaviour
///
/// If the node already has the [`NodeState::CONSTRAINED`] flag
/// set and the incoming constraint is unchanged, the entire
/// subtree is skipped. Otherwise the flag is set, the stored
/// constraint is updated, and propagation continues to children.
///
/// When the constraint changes, the [`NodeState::BUILT`] flag is
/// also cleared so the subsequent [`build`] pass re-measures the
/// node.
///
/// # Panics
///
/// Panics if `id` is not present in `nodes`.
pub fn constrain<
    Id,
    T: Rectree<Id = Id, Nodes = N>,
    N: RectNodes<Id = Id>,
>(
    tree: &T,
    nodes: &mut N,
    id: &T::Id,
    parent: Constraint,
) {
    let node = nodes.get_node(id).expect("constrain: Id is invalid!");

    let old_constraint = node.constraint;
    let constraint_unchanged = parent == old_constraint;

    if let Some(n) = nodes.get_node_mut(id) {
        // Skip the subtree if the constraint stays the same.
        if n.state.is_constrained() && constraint_unchanged {
            return;
        }

        n.state.has_reconstrained();

        n.constraint = parent;
        // Constraint changed means the built size is now stale.
        if !constraint_unchanged {
            n.state.needs_rebuild();
        }
    }

    // Derive this node's constraint from the parent's.
    let constraint = tree.constrain(id, nodes, parent);

    // Propagate the resolved constraint down to children.
    tree.for_each_child(id, nodes, |child, nodes| {
        constrain(tree, nodes, child, constraint);
    });
}

/// Recursively builds the layout tree bottom-up.
///
/// Children are built before their parent so that each parent
/// can read child sizes when computing its own size. After
/// measuring, the node's [`NodeState::BUILT`] flag is set and
/// its [`NodeState::POSITIONED`] flag is cleared because a new
/// size may require new child translations.
///
/// # Short-circuit behaviour
///
/// If the node's [`NodeState::BUILT`] flag is already set, the
/// entire subtree is skipped - its sizes and child translations
/// are still current.
///
/// # Panics
///
/// Panics if `id` is not present in `nodes`.
pub fn build<
    Id,
    T: Rectree<Id = Id, Nodes = N>,
    N: RectNodes<Id = Id>,
>(
    tree: &T,
    nodes: &mut N,
    id: &T::Id,
) {
    let node = nodes.get_node(id).expect("build: Id is invalid!");

    // Already up-to-date; skip this entire subtree.
    if node.state.is_built() {
        return;
    }

    let constraint = node.constraint;

    tree.for_each_child(id, nodes, |child, nodes| {
        build(tree, nodes, child);
    });

    // All children are now measured; measure self.
    let size = tree.build(id, constraint, nodes);

    if let Some(n) = nodes.get_node_mut(id) {
        n.size = size;
        n.state.needs_reposition();
        n.state.has_rebuilt();
    }
}

/// Re-measures a single node and walks upward if its size
/// changed.
///
/// Called after a child's size change has already been
/// recorded. Unlike [`build`], it does not recurse into
/// children. It assumes their sizes in `nodes` are current
/// and re-invokes [`Rectree::build`] on `id` to let it
/// re-measure from the current child sizes.
///
/// If the resulting size differs from the previous one, the
/// parent is re-measured recursively until the size stabilises
/// or a root is reached. Returns the highest node that was
/// re-measured, used as the start for [`propagate_translation`].
///
/// # Panics
///
/// Panics if `id` is not present in `nodes`.
pub fn build_up<
    Id: Copy,
    T: Rectree<Id = Id, Nodes = N>,
    N: RectNodes<Id = Id>,
>(
    tree: &T,
    nodes: &mut N,
    id: &T::Id,
) -> Id {
    let node = nodes.get_node(id).expect("build_up: Id is invalid!");

    let constraint = node.constraint;
    let old_size = node.size;
    let parent = nodes.get_node(id).and_then(|n| n.parent_id);

    let size = tree.build(id, constraint, nodes);

    if let Some(n) = nodes.get_node_mut(id) {
        n.size = size;
        n.state.needs_reposition();
        n.state.has_rebuilt();
    }

    if size != old_size
        && let Some(ref parent_id) = parent
    {
        return build_up(tree, nodes, parent_id);
    }

    *id
}

/// Propagates world-space translations top-down through the
/// subtree.
///
/// `parent_world` is the absolute world translation of `id`'s
/// parent (use [`Vec2::ZERO`] for root nodes). For each node
/// the world translation is computed as
/// `parent_world + node.translation` and stored in
/// `node.world_translation`.
///
/// # Short-circuit behaviour
///
/// If the node's [`NodeState::POSITIONED`] flag is already set,
/// the node and its entire subtree are skipped - their world
/// translations are still current.
///
/// # Panics
///
/// Panics if `id` is not present in `nodes`.
pub fn propagate_translation<
    Id,
    T: Rectree<Id = Id, Nodes = N>,
    N: RectNodes<Id = Id>,
>(
    tree: &T,
    nodes: &mut N,
    id: &T::Id,
    parent_world: Vec2,
    parent_translation_changed: bool,
) {
    let node = nodes
        .get_node(id)
        .expect("propagate_translation: Id is invalid!");

    // Already up-to-date; skip this entire subtree.
    if !parent_translation_changed && node.state.is_positioned() {
        return;
    }

    let world = parent_world + node.translation;

    let mut translation_changed = false;
    if let Some(n) = nodes.get_node_mut(id) {
        translation_changed = n.world_translation != world;
        if translation_changed {
            n.world_translation = world;
        }
        n.state.has_repositioned();
    }

    tree.for_each_child(id, nodes, |child, nodes| {
        propagate_translation(
            tree,
            nodes,
            child,
            world,
            translation_changed,
        );
    });
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::cell::Cell;

    use super::*;

    #[test]
    fn test_layout_full_pass() {
        let mut tree = WidgetTree::default();

        tree.add_column(0, None, vec![1, 2]);
        tree.add_fixed(1, Some(0), Size::new(10.0, 10.0));
        tree.add_fixed(2, Some(0), Size::new(20.0, 5.0));

        tree.layout(&0);

        // 3 constrains + 3 builds + 3 translation propagations.
        assert_eq!(tree.tree.for_each_child_calls.get(), 9);
        assert_eq!(tree.tree.constrain_calls.get(), 3);
        assert_eq!(tree.tree.build_calls.get(), 3);

        assert_eq!(tree.nodes.0[&1].size, Size::new(10.0, 10.0));
        assert_eq!(tree.nodes.0[&2].size, Size::new(20.0, 5.0));
        // Width = max(10, 20) = 20; Height = 10 + 5 = 15.
        assert_eq!(tree.nodes.0[&0].size, Size::new(20.0, 15.0));

        assert!(tree.nodes.0[&0].state.is_ready());
        assert!(tree.nodes.0[&1].state.is_ready());
        assert!(tree.nodes.0[&2].state.is_ready());

        let fec = tree.tree.for_each_child_calls.get();
        let cc = tree.tree.constrain_calls.get();
        let bc = tree.tree.build_calls.get();

        // Further layouts should have nothing rebuilt.
        tree.layout(&0);
        tree.layout(&1);
        tree.layout(&2);

        assert_eq!(tree.tree.for_each_child_calls.get(), fec);
        assert_eq!(tree.tree.constrain_calls.get(), cc);
        assert_eq!(tree.tree.build_calls.get(), bc);
    }

    #[test]
    fn test_constrain_stores_constraint() {
        let mut wt = WidgetTree::default();
        wt.add_fixed(0, None, Size::splat(10.0));

        let c = Constraint::tight(Size::splat(100.0));
        constrain(&wt.tree, &mut wt.nodes, &0, c);

        assert_eq!(wt.nodes.0[&0].constraint, c);
        assert!(wt.nodes.0[&0].state.is_constrained());
    }

    #[test]
    fn test_constrain_propagates_to_children() {
        let mut wt = WidgetTree::default();
        wt.add_column(0, None, vec![1]);
        wt.add_fixed(1, Some(0), Size::splat(10.0));

        let c = Constraint::loose(Size::splat(100.0));
        constrain(&wt.tree, &mut wt.nodes, &0, c);

        // Column passes constraint through unchanged.
        assert_eq!(wt.nodes.0[&1].constraint, c);
        assert_eq!(wt.tree.constrain_calls.get(), 2);
    }

    #[test]
    fn test_constrain_short_circuits_if_unchanged() {
        let mut wt = WidgetTree::default();
        wt.add_column(0, None, vec![1]);
        wt.add_fixed(1, Some(0), Size::splat(10.0));

        let c = Constraint::loose(Size::splat(100.0));
        constrain(&wt.tree, &mut wt.nodes, &0, c);
        let calls = wt.tree.constrain_calls.get();

        // Same constraint: entire subtree is skipped.
        constrain(&wt.tree, &mut wt.nodes, &0, c);
        assert_eq!(wt.tree.constrain_calls.get(), calls);
    }

    #[test]
    fn test_constrain_change_clears_built() {
        let mut wt = WidgetTree::default();
        wt.add_fixed(0, None, Size::splat(10.0));

        constrain(
            &wt.tree,
            &mut wt.nodes,
            &0,
            Constraint::loose(Size::splat(100.0)),
        );
        build(&wt.tree, &mut wt.nodes, &0);
        assert!(wt.nodes.0[&0].state.is_built());

        // A different constraint must clear the BUILT flag.
        constrain(
            &wt.tree,
            &mut wt.nodes,
            &0,
            Constraint::loose(Size::splat(200.0)),
        );
        assert!(!wt.nodes.0[&0].state.is_built());
    }

    #[test]
    fn test_build_sets_size() {
        let mut wt = WidgetTree::default();
        wt.add_fixed(0, None, Size::new(30.0, 20.0));
        build(&wt.tree, &mut wt.nodes, &0);

        assert_eq!(wt.nodes.0[&0].size, Size::new(30.0, 20.0));
        assert!(wt.nodes.0[&0].state.is_built());
    }

    #[test]
    fn test_build_column_sums_children() {
        let mut wt = WidgetTree::default();
        wt.add_column(0, None, vec![1, 2]);
        wt.add_fixed(1, Some(0), Size::new(10.0, 10.0));
        wt.add_fixed(2, Some(0), Size::new(20.0, 5.0));
        build(&wt.tree, &mut wt.nodes, &0);

        // Width = max(10, 20) = 20; Height = 10 + 5 = 15.
        assert_eq!(wt.nodes.0[&0].size, Size::new(20.0, 15.0));
    }

    #[test]
    fn test_build_short_circuits_if_built() {
        let mut wt = WidgetTree::default();
        wt.add_fixed(0, None, Size::splat(10.0));
        build(&wt.tree, &mut wt.nodes, &0);
        let calls = wt.tree.build_calls.get();

        // Already built; no further calls.
        build(&wt.tree, &mut wt.nodes, &0);
        assert_eq!(wt.tree.build_calls.get(), calls);
    }

    #[test]
    fn test_propagate_translation_sets_world_pos() {
        let mut wt = WidgetTree::default();
        wt.add_column(0, None, vec![1]);
        wt.add_fixed(1, Some(0), Size::splat(10.0));

        wt.nodes.0.get_mut(&1).unwrap().translation =
            Vec2::new(10.0, 5.0);
        propagate_translation(
            &wt.tree,
            &mut wt.nodes,
            &0,
            Vec2::ZERO,
            false,
        );

        assert_eq!(wt.nodes.0[&0].world_translation, Vec2::ZERO);
        assert_eq!(
            wt.nodes.0[&1].world_translation,
            Vec2::new(10.0, 5.0),
        );
    }

    #[test]
    fn test_propagate_translation_accumulates() {
        let mut wt = WidgetTree::default();
        wt.add_column(0, None, vec![1, 2]);
        wt.add_fixed(1, Some(0), Size::new(10.0, 10.0));
        wt.add_fixed(2, Some(0), Size::new(20.0, 5.0));

        // build positions children: node 1 at y=0, node 2 at y=10.
        build(&wt.tree, &mut wt.nodes, &0);
        propagate_translation(
            &wt.tree,
            &mut wt.nodes,
            &0,
            Vec2::ZERO,
            false,
        );

        assert_eq!(wt.nodes.0[&1].world_translation, Vec2::ZERO);
        assert_eq!(
            wt.nodes.0[&2].world_translation,
            Vec2::new(0.0, 10.0),
        );
    }

    #[test]
    fn test_propagate_translation_short_circuits_if_positioned() {
        let mut wt = WidgetTree::default();
        wt.add_column(0, None, vec![1]);
        wt.add_fixed(1, Some(0), Size::splat(10.0));

        propagate_translation(
            &wt.tree,
            &mut wt.nodes,
            &0,
            Vec2::ZERO,
            false,
        );
        let calls = wt.tree.for_each_child_calls.get();

        // All nodes POSITIONED; entire subtree is skipped.
        propagate_translation(
            &wt.tree,
            &mut wt.nodes,
            &0,
            Vec2::ZERO,
            false,
        );
        assert_eq!(wt.tree.for_each_child_calls.get(), calls);
    }

    type Id = usize;

    /// Flat node storage backed by a [`BTreeMap`].
    #[derive(Default)]
    struct Nodes(BTreeMap<Id, RectNode<Id>>);

    impl Nodes {
        fn add(&mut self, id: Id, parent: Option<Id>) {
            self.0.insert(id, RectNode::new(parent));
        }
    }

    impl RectNodes for Nodes {
        type Id = Id;

        fn get_node(&self, id: &Id) -> Option<&RectNode<Id>> {
            self.0.get(id)
        }

        fn get_node_mut(
            &mut self,
            id: &Id,
        ) -> Option<&mut RectNode<Id>> {
            self.0.get_mut(id)
        }
    }

    enum Widget {
        Column(Vec<Id>),
        Fixed(Size),
    }

    #[derive(Default)]
    struct Tree {
        widgets: BTreeMap<Id, Widget>,
        for_each_child_calls: Cell<u32>,
        constrain_calls: Cell<u32>,
        build_calls: Cell<u32>,
    }

    impl Rectree for Tree {
        type Id = Id;
        type Nodes = Nodes;

        fn for_each_child(
            &self,
            id: &Id,
            nodes: &mut Nodes,
            mut f: impl FnMut(&Id, &mut Nodes),
        ) {
            self.for_each_child_calls
                .set(self.for_each_child_calls.get() + 1);

            let Some(widget) = self.widgets.get(id) else {
                return;
            };

            if let Widget::Column(children) = widget {
                for child in children {
                    f(child, nodes);
                }
            }
        }

        fn constrain(
            &self,
            id: &Id,
            _nodes: &Nodes,
            parent: Constraint,
        ) -> Constraint {
            self.constrain_calls.set(self.constrain_calls.get() + 1);

            let widget =
                self.widgets.get(id).expect("Id is invalid!");

            match widget {
                Widget::Column(_) => parent,
                Widget::Fixed(size) => Constraint::tight(*size),
            }
        }

        fn build(
            &self,
            id: &Id,
            constraint: Constraint,
            nodes: &mut Nodes,
        ) -> Size {
            self.build_calls.set(self.build_calls.get() + 1);

            let widget =
                self.widgets.get(id).expect("Id is invalid!");

            let size = match widget {
                Widget::Column(children) => {
                    let mut size = Size::ZERO;
                    for child in children {
                        let child_size = nodes.get_size(child);
                        nodes.set_translation(
                            child,
                            Vec2::new(0.0, size.height),
                        );
                        size.width = size.width.max(child_size.width);
                        size.height += child_size.height;
                    }

                    size
                }
                Widget::Fixed(size) => *size,
            };

            constraint.constrain(size)
        }
    }

    #[derive(Default)]
    struct WidgetTree {
        tree: Tree,
        nodes: Nodes,
    }

    impl WidgetTree {
        pub fn add_fixed(
            &mut self,
            id: Id,
            parent: Option<Id>,
            size: Size,
        ) {
            self.tree.widgets.insert(id, Widget::Fixed(size));
            self.nodes.add(id, parent);
        }

        pub fn add_column(
            &mut self,
            id: Id,
            parent: Option<Id>,
            column: Vec<Id>,
        ) {
            self.tree.widgets.insert(id, Widget::Column(column));
            self.nodes.add(id, parent);
        }

        pub fn layout(&mut self, id: &Id) {
            layout(&self.tree, &mut self.nodes, id);
        }
    }
}

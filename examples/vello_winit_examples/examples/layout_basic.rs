//! # layout_basic
//!
//! This example demonstrates how to wire up the `rectree` layout
//! crate into a real application.
//!
//! ## How rectree layout works
//!
//! rectree uses a three-pass algorithm, applied top-down then
//! bottom-up:
//!
//! 1. **Constrain** (top-down): each node receives its parent's
//!    constraint and narrows it for its children.  The resolved
//!    `Constraint` is stored on the node.
//!
//! 2. **Build** (bottom-up): children are measured before parents.
//!    A node calls `Widget::build`, which reads child *sizes* and
//!    writes child *translations*, then returns the node's own size.
//!    Sizes flow upward; positions flow downward within the same
//!    pass.
//!
//! 3. **Propagate translation** (top-down): accumulates local
//!    translations into absolute `world_translation` values used
//!    for rendering.
//!
//! ## Split between `World` and `Nodes`
//!
//! rectree requires two separate objects at call sites:
//!
//! - `&T: LayoutTree` — owns the *tree structure* and widget logic
//!   (read-only during layout).
//! - `&mut N: LayoutNode` — owns the *per-node data* (constraint,
//!   size, translation) and is mutated by the layout passes.
//!
//! This split is necessary because Rust cannot hold `&T` and
//! `&mut T` at the same time when `T == N`.  Here `World` is `T`
//! and `Nodes` is `N`.

use std::any::Any;

use hashbrown::HashMap;
use kurbo::{Affine, Circle, Point, Rect, Size as KSize, Stroke};
use rectree::{
    Constraint, NodeContext, RectNode, RectNodes, Rectree, Size,
    Vec2, layout,
};
use vello::Scene;
use vello::peniko::Color;
use vello::peniko::color::palette::css;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();
    let root_size = demo.nodes.window_size;
    let mut builder = demo.builder();

    // Helper closure: builds a vertical column of seven colored
    // boxes with a diamond-shaped height profile (40→100→40 px).
    let create_column = |b: &mut Builder| {
        Vertical::new(10.0).show(b, |b| {
            const WIDTH: f32 = 200.0;
            vec![
                FixedSizeWidget::new(Size::new(WIDTH, 40.0))
                    .with_color(css::RED)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 60.0))
                    .with_color(css::ORANGE)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 80.0))
                    .with_color(css::YELLOW)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 100.0))
                    .with_color(css::GREEN)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 80.0))
                    .with_color(css::BLUE)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 60.0))
                    .with_color(css::VIOLET)
                    .show(b),
                FixedSizeWidget::new(Size::new(WIDTH, 40.0))
                    .with_color(css::PURPLE)
                    .show(b),
            ]
        })
    };

    // Build the widget tree using the declarative `Builder` API.
    // Each `show` / `show_with_child` call allocates a `NodeId`,
    // registers the widget in `World`, and records the parent-child
    // relationship so `LayoutTree::children` can traverse it.
    let root_id = FixedSizeWidget::new(root_size).show_with_child(
        &mut builder,
        |b| {
            PlaceWidget::show(
                Alignment::Both {
                    h: HAlign::Center,
                    v: VAlign::Horizon,
                },
                b,
                |b| {
                    Padding::all(20.0).show(b, |b| {
                        Vertical::new(20.0).show(b, |b| {
                            const HEIGHT: f32 = 60.0;
                            vec![
                                Horizontal::new(50.0).show(b, |b| {
                                    vec![
                                        create_column(b),
                                        create_column(b),
                                        create_column(b),
                                    ]
                                }),
                                FixedSizeWidget::new(Size::new(
                                    50.0, HEIGHT,
                                ))
                                .with_color(css::CYAN)
                                .show(b),
                                FixedSizeWidget::new(Size::new(
                                    200.0, HEIGHT,
                                ))
                                .with_color(css::SALMON)
                                .show(b),
                                FixedSizeWidget::new(Size::new(
                                    800.0, HEIGHT,
                                ))
                                .with_color(css::RED)
                                .show(b),
                            ]
                        })
                    })
                },
            );
        },
    );

    demo.root_id = Some(root_id);

    // Run the initial layout before opening the window.
    demo.layout();

    let mut app = VelloWinitApp::new(demo);
    event_loop.run_app(&mut app).unwrap();
}

// ---------------------------------------------------------------------------
// ID type
// ---------------------------------------------------------------------------

/// Opaque handle that identifies a single node.
///
/// Must be `Copy + Eq + Hash` so rectree can use it as a map key
/// and pass it around without cloning.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub struct NodeId(u32);

// ---------------------------------------------------------------------------
// Node storage  (`N: LayoutNode`)
// ---------------------------------------------------------------------------

/// Flat storage for every node's layout data.
///
/// This is the mutable half of the layout split.  It only holds
/// per-node numbers (`constraint`, `size`, `translation`, …) — it
/// knows nothing about the widget logic or tree structure.
///
/// `rectree` mutates this through the [`LayoutNode`] trait.
pub struct Nodes {
    data: HashMap<NodeId, RectNode<NodeId>>,
    next_id: u32,
    pub window_size: Size,
}

impl Nodes {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
            next_id: 0,
            window_size: Size::new(800.0, 600.0),
        }
    }

    /// Allocate a new node, recording its parent link so the
    /// bottom-up `build` pass can walk upward when a size changes.
    fn insert(&mut self, parent: Option<NodeId>) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.data.insert(id, RectNode::new(parent));
        id
    }

    /// Clear all `NodeState` flags on one node so the next
    /// `layout()` call re-runs all three passes for its subtree.
    fn invalidate(&mut self, id: NodeId) {
        if let Some(n) = self.data.get_mut(&id) {
            n.state.reset();
        }
    }
}

/// Implement `LayoutNode` by forwarding to the flat `data` map.
/// rectree only ever calls `get_node` / `get_node_mut` through this
/// trait, keeping the storage details encapsulated.
impl RectNodes for Nodes {
    type Id = NodeId;

    fn get_node(&self, id: &NodeId) -> Option<&RectNode<NodeId>> {
        self.data.get(id)
    }

    fn get_node_mut(
        &mut self,
        id: &NodeId,
    ) -> Option<&mut RectNode<NodeId>> {
        self.data.get_mut(id)
    }
}

// ---------------------------------------------------------------------------
// Tree structure + widget logic  (`T: LayoutTree`)
// ---------------------------------------------------------------------------

/// The read-only half of the layout split.
///
/// `World` owns:
/// - the widget instances (their logic),
/// - the parent→children mapping (tree structure).
///
/// It is passed as `&World` to the rectree free functions, while
/// `&mut Nodes` is passed separately — avoiding the borrow conflict
/// that would arise if a single type owned both.
pub struct World {
    widgets: HashMap<NodeId, Box<dyn Widget>>,
    /// Maps every node to its ordered list of children.
    children: HashMap<NodeId, Vec<NodeId>>,
    /// Nodes that have no parent (typically one: the window root).
    roots: Vec<NodeId>,
}

impl World {
    fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            children: HashMap::new(),
            roots: Vec::new(),
        }
    }

    /// Register a new node in the tree.  Called by [`Builder`]
    /// immediately after `Nodes::insert` so both halves stay in
    /// sync.
    fn add_node(&mut self, id: NodeId, parent: Option<NodeId>) {
        self.children.entry(id).or_default();
        if let Some(p) = parent {
            self.children.entry(p).or_default().push(id);
        } else {
            self.roots.push(id);
        }
    }
}

/// Implement `LayoutTree` by delegating to the stored widget
/// instances.  rectree calls these methods during the constrain and
/// build passes.
impl Rectree for World {
    type Id = NodeId;
    type Nodes = Nodes;

    /// Returns the children of `id` in insertion order.
    fn children<'a>(
        &'a self,
        id: &NodeId,
    ) -> impl IntoIterator<Item = &'a NodeId> {
        self.children.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Asks the widget to derive the node's own constraint from its
    /// parent's.  Most widgets pass through unchanged; containers
    /// like `PaddingWidget` subtract their insets.
    fn constrain(
        &self,
        id: &NodeId,
        parent: Constraint,
    ) -> Constraint {
        self.widgets
            .get(id)
            .map(|w| w.constraint(parent))
            .unwrap_or(parent)
    }

    /// Asks the widget to measure itself and position its children.
    /// Children have already been built by the time this is called
    /// (bottom-up order), so their sizes are available via `nodes`.
    ///
    /// Widgets can *read* child sizes and *write* child translations
    /// — but cannot mutate child sizes directly.
    fn build(
        &self,
        id: &NodeId,
        constraint: Constraint,
        nodes: &mut Nodes,
    ) -> Size {
        self.widgets
            .get(id)
            .map(|w| w.build(constraint, nodes))
            .unwrap_or(Size::ZERO)
    }
}

// ---------------------------------------------------------------------------
// Widget trait
// ---------------------------------------------------------------------------

/// A widget defines *how* a node behaves during layout.
///
/// - [`constraint`](Widget::constraint): narrows the parent's
///   `Constraint` for this node's children (e.g. subtract padding).
/// - [`build`](Widget::build): given the node's own constraint and
///   the already-built children, compute this node's `Size` and set
///   children's local translations.
pub trait Widget: Any {
    /// Default: pass the parent constraint through unchanged.
    fn constraint(&self, parent: Constraint) -> Constraint {
        parent
    }

    fn build(
        &self,
        constraint: Constraint,
        nodes: &mut Nodes,
    ) -> Size;
}

// ---------------------------------------------------------------------------
// LayoutDemo
// ---------------------------------------------------------------------------

pub struct LayoutDemo {
    /// Read-only tree: widget logic + parent-child relationships.
    world: World,
    /// Mutable storage: per-node layout numbers.
    nodes: Nodes,
    /// The root `NodeId`; stored so `size_changed` can update it.
    root_id: Option<NodeId>,
}

impl LayoutDemo {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            nodes: Nodes::new(),
            root_id: None,
        }
    }

    pub fn builder(&mut self) -> Builder<'_> {
        Builder {
            world: &mut self.world,
            nodes: &mut self.nodes,
            parent_id: None,
        }
    }

    /// Run all three layout passes for every root node.
    ///
    /// `layout` (the rectree free function) runs constrain→build→
    /// propagate_translation internally and short-circuits each pass
    /// via `NodeState` flags — so re-calling this every frame is
    /// cheap when nothing changed.
    fn layout(&mut self) {
        for root in self.world.roots.iter() {
            // Reset the root so the passes start fresh.  Children
            // are only re-processed when their constraint or size
            // actually changes, thanks to `NodeState` guards.
            self.nodes.get_node_mut(root).unwrap().state.reset();
            layout(&self.world, &mut self.nodes, root);
        }
    }

    /// Walk the node tree and draw each node's bounding box.
    ///
    /// - Colored `FixedSizeWidget`s are filled with their color.
    /// - Every node gets a white stroke so the layout boxes are
    ///   visible.
    /// - A small red dot marks each node's origin point.
    fn draw_tree(&self, scene: &mut Scene, transform: Affine) {
        for root_id in &self.world.roots {
            // Iterative DFS using a stack to avoid recursion limits.
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                let Some(node) = self.nodes.get_node(&node_id) else {
                    continue;
                };

                // `world_translation` is set by `propagate_translation`
                // and holds the node's absolute position in window space.
                let world_pos = node.world_translation;
                let size = node.size;
                let world_rect = Rect::from_origin_size(
                    Point::new(
                        world_pos.x as f64,
                        world_pos.y as f64,
                    ),
                    KSize::new(size.width as f64, size.height as f64),
                );

                // Only `FixedSizeWidget`s carry a fill color; other
                // nodes are transparent (only their border is drawn).
                if let Some(color) =
                    self.world.widgets.get(&node_id).and_then(
                        |widget| {
                            let widget: &dyn Any = widget.as_ref();
                            widget
                                .downcast_ref::<FixedSizeWidget>()
                                .map(|f| f.color)
                        },
                    )
                {
                    scene.fill(
                        vello::peniko::Fill::NonZero,
                        transform,
                        color,
                        None,
                        &world_rect,
                    );
                }

                // White border shows the layout box of every node.
                scene.stroke(
                    &Stroke::new(2.0),
                    transform,
                    Color::WHITE,
                    None,
                    &world_rect,
                );

                // Red dot at the node's top-left origin.
                let origin = Circle::new(world_rect.origin(), 5.0);
                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    css::RED,
                    None,
                    &origin,
                );

                if let Some(children) =
                    self.world.children.get(&node_id)
                {
                    for child_id in children {
                        stack.push(*child_id);
                    }
                }
            }
        }
    }
}

impl Default for LayoutDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl VelloDemo for LayoutDemo {
    fn window_title(&self) -> &'static str {
        "Layout Showcase"
    }

    fn initial_logical_size(&self) -> (f64, f64) {
        (
            self.nodes.window_size.width as f64,
            self.nodes.window_size.height as f64,
        )
    }

    /// Called by the harness whenever the window is resized.
    ///
    /// We update the root `FixedSizeWidget`'s size to match the new
    /// window dimensions and invalidate the root node so the next
    /// `layout()` call re-runs the passes from the top.
    fn size_changed(&mut self, size: Size) {
        self.nodes.window_size = size;

        let Some(root_id) = self.root_id else { return };

        if let Some(widget) = self.world.widgets.get_mut(&root_id)
            && let Some(fixed_widget) = (widget.as_mut()
                as &mut dyn Any)
                .downcast_mut::<FixedSizeWidget>()
        {
            fixed_widget.size = size;
            self.nodes.invalidate(root_id);
        }
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut Scene,
        scale_factor: f64,
    ) {
        self.layout();
        self.draw_tree(scene, Affine::scale(scale_factor));
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Accumulates nodes into `World` and `Nodes` during tree
/// construction.
///
/// `parent_id` tracks the current insertion point; each `add_widget`
/// call creates a child under it and recurses with itself as the new
/// parent, producing a depth-first construction order.
pub struct Builder<'a> {
    world: &'a mut World,
    nodes: &'a mut Nodes,
    /// The node that newly created nodes will be children of.
    /// `None` means the next node becomes a root.
    parent_id: Option<NodeId>,
}

impl Builder<'_> {
    /// Create a node, run `add_content` to build its children, then
    /// return the node's `NodeId`.
    ///
    /// `add_content` is a closure that receives a `Builder` already
    /// scoped to the new node as its parent, so any widgets created
    /// inside it automatically become children.
    pub fn add_widget<W: Widget + 'static>(
        &mut self,
        add_content: impl FnOnce(&mut Builder) -> W,
    ) -> NodeId {
        let id = self.nodes.insert(self.parent_id);
        self.world.add_node(id, self.parent_id);

        let w = Box::new(add_content(&mut Builder {
            world: self.world,
            nodes: self.nodes,
            parent_id: Some(id),
        }));
        self.world.widgets.insert(id, w);

        id
    }
}

// ---------------------------------------------------------------------------
// Demo widgets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum VAlign {
    Top,
    /// Vertically centered.
    Horizon,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Both { h: HAlign, v: VAlign },
    Horizontal(HAlign),
    Vertical(VAlign),
}

/// Positions its single child within the available space according
/// to an [`Alignment`].
///
/// Does not contribute any size itself (`Size::ZERO`) — it is purely
/// a positioning container.
pub struct PlaceWidget {
    pub alignment: Alignment,
    pub child: NodeId,
}

impl PlaceWidget {
    pub fn show(
        alignment: Alignment,
        b: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> NodeId,
    ) -> NodeId {
        b.add_widget(|b| {
            let child = add_content(b);
            PlaceWidget { alignment, child }
        })
    }
}

impl Widget for PlaceWidget {
    fn build(
        &self,
        constraint: Constraint,
        nodes: &mut Nodes,
    ) -> Size {
        let child_size = nodes.get_size(&self.child);

        // The available space is the maximum of our own constraint.
        let avail_w = constraint.max.width;
        let avail_h = constraint.max.height;

        let (halign, valign) = match self.alignment {
            Alignment::Both { h, v } => (Some(h), Some(v)),
            Alignment::Horizontal(h) => (Some(h), None),
            Alignment::Vertical(v) => (None, Some(v)),
        };

        let x = match halign {
            Some(HAlign::Left) => 0.0,
            Some(HAlign::Center) => {
                (avail_w - child_size.width) / 2.0
            }
            Some(HAlign::Right) => avail_w - child_size.width,
            None => 0.0,
        };
        let y = match valign {
            Some(VAlign::Top) => 0.0,
            Some(VAlign::Horizon) => {
                (avail_h - child_size.height) / 2.0
            }
            Some(VAlign::Bottom) => avail_h - child_size.height,
            None => 0.0,
        };

        // Write the child's local translation.  `propagate_translation`
        // will later accumulate this into an absolute world position.
        nodes.set_translation(&self.child, Vec2::new(x, y));

        Size::ZERO
    }
}

/// Builder for [`HorizontalWidget`].
#[derive(Debug, Clone)]
pub struct Horizontal {
    pub spacing: f32,
}

impl Horizontal {
    pub fn new(spacing: f32) -> Self {
        Self { spacing }
    }

    pub fn show(
        self,
        builder: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> Vec<NodeId>,
    ) -> NodeId {
        builder.add_widget(|b| HorizontalWidget {
            style: self,
            children: add_content(b),
        })
    }
}

/// Lays out children left-to-right with uniform spacing.
///
/// The children's `NodeId`s are stored directly on the widget so
/// `build` can iterate them without going through the tree.
#[derive(Debug, Clone)]
pub struct HorizontalWidget {
    pub style: Horizontal,
    pub children: Vec<NodeId>,
}

impl Widget for HorizontalWidget {
    fn build(
        &self,
        constraint: Constraint,
        nodes: &mut Nodes,
    ) -> Size {
        let mut height = 0.0;
        let mut width = 0.0;

        for child_id in &self.children {
            // Children are already built (bottom-up), so their
            // sizes are final.
            let child_size = nodes.get_size(child_id);

            // Place this child at the current x cursor.
            nodes.set_translation(child_id, Vec2::new(width, 0.0));

            width += child_size.width + self.style.spacing;
            height = child_size.height.max(height);
        }
        // Strip the trailing gap added after the last child.
        if !self.children.is_empty() {
            width -= self.style.spacing;
        }

        // `Constraint::constrain` clamps the intrinsic size to the
        // min/max bounds, so the widget respects its constraint.
        constraint.constrain(Size::new(width, height))
    }
}

/// Builder for [`VerticalWidget`].
#[derive(Debug, Clone)]
pub struct Vertical {
    pub spacing: f32,
}

impl Vertical {
    pub fn new(spacing: f32) -> Self {
        Self { spacing }
    }

    pub fn show(
        self,
        builder: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> Vec<NodeId>,
    ) -> NodeId {
        builder.add_widget(|b| VerticalWidget {
            style: self,
            children: add_content(b),
        })
    }
}

/// Lays out children top-to-bottom with uniform spacing.
#[derive(Debug, Clone)]
pub struct VerticalWidget {
    pub style: Vertical,
    pub children: Vec<NodeId>,
}

impl Widget for VerticalWidget {
    fn build(
        &self,
        constraint: Constraint,
        nodes: &mut Nodes,
    ) -> Size {
        let mut width = 0.0;
        let mut height = 0.0;

        for child_id in &self.children {
            let child_size = nodes.get_size(child_id);

            nodes.set_translation(child_id, Vec2::new(0.0, height));

            height += child_size.height + self.style.spacing;
            width = child_size.width.max(width);
        }
        if !self.children.is_empty() {
            height -= self.style.spacing;
        }

        constraint.constrain(Size::new(width, height))
    }
}

/// Builder for [`PaddingWidget`].
#[derive(Debug, Clone, Copy)]
pub struct Padding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Padding {
    fn all(padding: f32) -> Self {
        Self {
            left: padding,
            right: padding,
            top: padding,
            bottom: padding,
        }
    }

    fn show(
        self,
        builder: &mut Builder,
        add_content: impl FnOnce(&mut Builder) -> NodeId,
    ) -> NodeId {
        builder.add_widget(|b| PaddingWidget {
            style: self,
            child: add_content(b),
        })
    }
}

/// Wraps a single child with configurable insets on each side.
///
/// The insets are applied in two places:
/// - `constraint`: subtracts them from the available space so the
///   child doesn't overflow.
/// - `build`: offsets the child's translation inward and grows the
///   returned size to include the insets.
#[derive(Debug)]
pub struct PaddingWidget {
    pub style: Padding,
    pub child: NodeId,
}

impl Widget for PaddingWidget {
    /// Reduce the parent constraint by the padding amounts so the
    /// child is told it has less space to fill.
    fn constraint(&self, parent: Constraint) -> Constraint {
        let h_pad = self.style.left + self.style.right;
        let v_pad = self.style.top + self.style.bottom;

        Constraint {
            min: Size::ZERO,
            max: Size {
                width: (parent.max.width - h_pad).max(0.0),
                height: (parent.max.height - v_pad).max(0.0),
            },
        }
    }

    fn build(
        &self,
        _constraint: Constraint,
        nodes: &mut Nodes,
    ) -> Size {
        let child_size = nodes.get_size(&self.child);

        // Shift the child inward by the padding amounts.
        nodes.set_translation(
            &self.child,
            Vec2::new(self.style.left, self.style.top),
        );

        // Our own size wraps the child plus the padding on both
        // sides.
        Size::new(
            child_size.width + self.style.left + self.style.right,
            child_size.height + self.style.top + self.style.bottom,
        )
    }
}

/// A leaf widget that returns a fixed size regardless of the
/// constraint passed in from its parent.
///
/// Used both as the window root (to propagate the window size as a
/// tight constraint downward) and as colored leaf boxes in the demo.
#[derive(Debug, Clone)]
pub struct FixedSizeWidget {
    pub size: Size,
    pub color: Color,
}

impl FixedSizeWidget {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            color: Color::TRANSPARENT,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Show as a leaf node (no children).
    pub fn show(self, b: &mut Builder) -> NodeId {
        b.add_widget(|_| self)
    }

    /// Show with an inner subtree; the children are built by
    /// `add_content` before the widget is constructed.
    pub fn show_with_child(
        self,
        b: &mut Builder,
        add_content: impl FnOnce(&mut Builder),
    ) -> NodeId {
        b.add_widget(|b| {
            add_content(b);
            self
        })
    }
}

impl Widget for FixedSizeWidget {
    /// Override the parent constraint entirely with a tight box
    /// around `self.size`.  Children (if any) will be told they
    /// have exactly this much space.
    fn constraint(&self, _parent: Constraint) -> Constraint {
        Constraint::tight(self.size)
    }

    fn build(
        &self,
        _constraint: Constraint,
        _nodes: &mut Nodes,
    ) -> Size {
        self.size
    }
}

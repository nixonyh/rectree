use std::any::Any;

use hashbrown::HashMap;
use kurbo::{Affine, Circle, Point, Rect, Size as KSize, Stroke};
use rectree::layout::{LayoutWorld, Positioner};
use rectree::{Constraint, NodeId, RectNode, Rectree, Size, Vec2};
use vello::Scene;
use vello::peniko::Color;
use vello::peniko::color::palette::css;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();
    let root_size = demo.window_size;
    let mut builder = demo.builder();

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

    let root_id =
        FixedSizeWidget::new(root_size)
        .show_with_child(&mut builder, |b| {
            PlaceWidget::new(Alignment::Both {
                h: HAlign::Center,
                v: VAlign::Horizon,
            })
            .show(b, |b| {
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
                });
            });
        });

    // Store the root ID for future reference.
    demo.root_id = Some(root_id);

    // Initial layout.
    demo.tree.layout(&demo.world);

    let mut app = VelloWinitApp::new(demo);

    event_loop.run_app(&mut app).unwrap();
}

pub struct World {
    widgets: HashMap<NodeId, Box<dyn Widget>>,
}

impl World {
    fn new() -> Self {
        Self {
            widgets: HashMap::new(),
        }
    }
}

impl LayoutWorld for World {
    fn constraint(
        &self,
        id: &NodeId,
        parent: Constraint,
    ) -> Constraint {
        self.widgets
            .get(id)
            .map(|w| w.constraint(parent))
            .unwrap_or(parent)
    }

    fn build(
        &self,
        id: &NodeId,
        node: &RectNode,
        tree: &Rectree,
        pos: &mut Positioner,
    ) -> Size {
        self.widgets
            .get(id)
            .map(|w| w.build(node, tree, pos))
            .unwrap_or(Size::ZERO)
    }
}

pub trait Widget: Any {
    fn constraint(&self, parent: Constraint) -> Constraint {
        parent
    }

    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size;
}

pub struct LayoutDemo {
    tree: Rectree,
    world: World,
    window_size: Size,
    root_id: Option<NodeId>,
}

pub struct Builder<'a> {
    pub demo: &'a mut LayoutDemo,
    pub parent_id: Option<NodeId>,
}

impl Builder<'_> {
    pub fn add_widget<W: Widget + 'static>(
        &mut self,
        add_content: impl FnOnce(&mut Builder) -> W,
    ) -> NodeId {
        let mut node = RectNode::new();
        if let Some(parent_id) = self.parent_id {
            node = node.with_parent(parent_id);
        }
        let id = self.demo.tree.insert(node);

        let w = Box::new(add_content(&mut Builder {
            demo: self.demo,
            parent_id: Some(id),
        }));
        self.demo.world.widgets.insert(id, w);

        id
    }
}

impl LayoutDemo {
    pub fn new() -> Self {
        Self {
            tree: Rectree::new(),
            world: World::new(),
            window_size: Size::new(800.0, 600.0),
            root_id: None,
        }
    }

    pub fn builder(&mut self) -> Builder<'_> {
        Builder {
            demo: self,
            parent_id: None,
        }
    }

    fn draw_tree(&self, scene: &mut Scene, transform: Affine) {
        // Start traversal from the root IDs provided by the tree.
        for root_id in self.tree.root_ids() {
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                // Get node from tree.
                let node = self.tree.get(&node_id);

                // Convert layout types to kurbo for rendering.
                let world_pos = node.world_translation();
                let size = node.size();
                let world_rect = Rect::from_origin_size(
                    Point::new(
                        world_pos.x as f64,
                        world_pos.y as f64,
                    ),
                    KSize::new(
                        size.width as f64,
                        size.height as f64,
                    ),
                );

                // Hack to get the color of `FixedSizeWidget`.
                // In real world scenario, you would want to
                // implement a `draw` method for your `Widget` trait.
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

                scene.stroke(
                    &Stroke::new(2.0),
                    transform,
                    Color::WHITE,
                    None,
                    &world_rect,
                );

                // Origin markers.
                let origin = Circle::new(world_rect.origin(), 5.0);

                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    css::RED,
                    None,
                    &origin,
                );

                // Traverse to children.
                for child_id in node.children().iter() {
                    stack.push(*child_id);
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
            self.window_size.width as f64,
            self.window_size.height as f64,
        )
    }

    fn size_changed(&mut self, size: Size) {
        self.window_size = size;

        // Propagate size change to the root widget.
        let Some(root_id) = self.root_id else { return };

        let Some(widget) = self.world.widgets.get_mut(&root_id)
        else {
            return;
        };

        if let Some(fixed_widget) = (widget.as_mut() as &mut dyn Any)
            .downcast_mut::<FixedSizeWidget>()
        {
            fixed_widget.size = size;
            // Trigger relayout for the root.
            self.tree.schedule_relayout(root_id);
        }
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut Scene,
        scale_factor: f64,
    ) {
        // Perform layouting.
        self.tree.layout(&self.world);

        self.draw_tree(scene, Affine::scale(scale_factor));
    }
}

// Below are some demo widgets to demonstrate how a UI library could
// potentially use `rectree` as a backend!

#[derive(Debug, Clone, Copy)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum VAlign {
    Top,
    Horizon,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Both { h: HAlign, v: VAlign },
    Horizontal(HAlign),
    Vertical(VAlign),
}

/// Place the child widget in a certain alignment.
pub struct PlaceWidget {
    pub alignment: Alignment,
}

impl PlaceWidget {
    pub fn new(alignment: Alignment) -> Self {
        Self { alignment }
    }

    pub fn show(
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

impl Widget for PlaceWidget {
    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let constraint = node.parent_constraint();
        let (halign, valign) = match self.alignment {
            Alignment::Both { h, v } => (Some(h), Some(v)),
            Alignment::Horizontal(halign) => (Some(halign), None),
            Alignment::Vertical(valign) => (None, Some(valign)),
        };

        for (id, child) in
            node.children().iter().map(|id| (id, tree.get(id)))
        {
            let child_size = child.size();
            let mut translation = Vec2::ZERO;
            let mut should_position = false;

            if let Some(halign) = halign
                && constraint.max.width.is_finite()
            {
                let width = constraint.max.width;
                should_position = true;
                translation.x = match halign {
                    HAlign::Left => 0.0,
                    HAlign::Center => {
                        width * 0.5 - child_size.width * 0.5
                    }
                    HAlign::Right => width - child_size.width,
                };
            }

            if let Some(valign) = valign
                && constraint.max.height.is_finite()
            {
                let height = constraint.max.height;
                should_position = true;
                translation.y = match valign {
                    VAlign::Top => 0.0,
                    VAlign::Horizon => {
                        height * 0.5 - child_size.height * 0.5
                    }
                    VAlign::Bottom => height - child_size.height,
                };
            }

            if should_position {
                positioner.set(*id, translation);
            }
        }

        // Placing the widget should not allocate any size.
        Size::ZERO
    }
}

/// [`HorizontalWidget`] builder.
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

/// Horizontal layout widget.
#[derive(Debug, Clone)]
pub struct HorizontalWidget {
    pub style: Horizontal,
    pub children: Vec<NodeId>,
}

impl Widget for HorizontalWidget {
    fn build(
        &self,
        _node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let mut max_height = 0.0;
        let mut x_cursor = 0.0;

        for id in self.children.iter() {
            let child_node = tree.get(id);
            let child_size = child_node.size();

            positioner.set(*id, Vec2::new(x_cursor, 0.0));
            x_cursor += child_size.width + self.style.spacing;

            // Track the tallest child.
            if child_size.height > max_height {
                max_height = child_size.height;
            }
        }
        // Remove the last added spacing.
        if !self.children.is_empty() {
            x_cursor -= self.style.spacing;
        }

        Size::new(x_cursor, max_height)
    }
}

/// [`VerticalWidget`] builder.
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

/// Vertical layout widget.
#[derive(Debug, Clone)]
pub struct VerticalWidget {
    pub style: Vertical,
    pub children: Vec<NodeId>,
}

impl Widget for VerticalWidget {
    fn build(
        &self,
        _node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let mut max_width = 0.0;
        let mut y_cursor = 0.0;

        for id in self.children.iter() {
            let child_node = tree.get(id);
            let child_size = child_node.size();

            positioner.set(*id, Vec2::new(0.0, y_cursor));

            y_cursor += child_size.height + self.style.spacing;
            // Track the widest child.
            if child_size.width > max_width {
                max_width = child_size.width;
            }
        }
        // Remove the last added spacing.
        if !self.children.is_empty() {
            y_cursor -= self.style.spacing;
        }

        Size::new(max_width, y_cursor)
    }
}

/// [`PaddingWidget`] builder.
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

/// A container widget that applies specific padding to each side.
#[derive(Debug)]
pub struct PaddingWidget {
    pub style: Padding,
    pub child: NodeId,
}

impl Widget for PaddingWidget {
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
        _node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let Padding {
            left,
            right,
            top,
            bottom,
        } = self.style;

        let child_node = tree.get(&self.child);
        let child_size = child_node.size();

        // Position the child with the specified padding offsets.
        positioner.set(self.child, Vec2::new(left, top));

        Size::new(
            child_size.width + left + right,
            child_size.height + top + bottom,
        )
    }
}

/// A widget that forces a specific size that ignores parent constraints.
#[derive(Debug, Clone)]
pub struct FixedSizeWidget {
    pub size: Size,
    pub color: Color,
}

impl Widget for FixedSizeWidget {
    fn constraint(&self, _parent: Constraint) -> Constraint {
        // Fixed size yields a tight constraint.
        Constraint::tight(self.size)
    }

    fn build(
        &self,
        _node: &RectNode,
        _tree: &Rectree,
        _positioner: &mut Positioner,
    ) -> Size {
        self.size
    }
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

    pub fn show(self, b: &mut Builder) -> NodeId {
        b.add_widget(|_| self)
    }

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

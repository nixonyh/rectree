use std::any::Any;

use hashbrown::HashMap;
use kurbo::{Affine, Circle, Rect, Size, Stroke, Vec2};
use rectree::layout::{LayoutSolver, LayoutWorld, Positioner};
use rectree::node::RectNode;
use rectree::{NodeId, Rectree};
use vello::Scene;
use vello::peniko::Color;
use vello_winit_examples::{VelloDemo, VelloWinitApp};
use winit::event_loop::EventLoop;

const AREA: f64 = 2500.0;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut demo = LayoutDemo::new();

    demo.add_widget(
        None,
        Color::from_rgb8(200, 200, 10),
        |demo, id| VerticalCenteredList {
            padding: 20.0,
            children: (0..5)
                .map(|_i| {
                    let area = FixedArea {
                        use_width: false,
                        target_area: AREA,
                    };

                    demo.add_widget(
                        Some(id),
                        Color::from_rgb8(10, 200, 200),
                        |_, _| area,
                    )
                })
                .collect(),
        },
    );
    // Initial layout.
    demo.tree.layout(&demo.world);

    let mut app = VelloWinitApp::new(demo);

    event_loop.run_app(&mut app).unwrap();
}

struct World {
    widgets: HashMap<NodeId, Box<dyn Widget>>,
    node_colors: HashMap<NodeId, Color>,
}

impl World {
    pub fn new() -> Self {
        Self {
            widgets: HashMap::new(),
            node_colors: HashMap::new(),
        }
    }
}

impl LayoutWorld for World {
    fn get_solver(&self, id: &NodeId) -> &dyn LayoutSolver {
        &**self.widgets.get(id).unwrap()
    }
}

#[derive(Debug, Clone)]
struct VerticalCenteredList {
    padding: f64,
    // In real world scenario, this should just store the ids
    // mapping to an arena of widgets.
    children: Vec<NodeId>,
}

impl LayoutSolver for VerticalCenteredList {
    fn build(
        &self,
        node: &RectNode,
        tree: &Rectree,
        positioner: &mut Positioner,
    ) -> Size {
        let width =
            node.parent_constraint().width.unwrap_or_else(|| {
                let mut max_width = 0.0;

                for id in self.children.iter() {
                    let node = tree.get(id);
                    max_width = node.size().width.max(max_width);
                }

                max_width
            }) + self.padding * 2.0;

        let mut height = self.padding;
        for id in self.children.iter() {
            let node = tree.get(id);
            let size = node.size();
            let remainder = width - size.width;

            let x = remainder * 0.5;
            let y = height;
            positioner.set(*id, Vec2::new(x, y));

            height += size.height + self.padding;
        }

        Size::new(width, height)
    }
}

#[derive(Debug, Clone, Copy)]
struct FixedArea {
    /// Use width if constrained on both axis.
    /// (Acts like a text widget.)
    pub use_width: bool,
    pub target_area: f64,
}

impl LayoutSolver for FixedArea {
    fn build(
        &self,
        node: &RectNode,
        _: &Rectree,
        _: &mut Positioner,
    ) -> Size {
        let constraint = node.parent_constraint();
        match (constraint.width, constraint.height) {
            (None, None) => {
                // Square
                Size::splat(self.target_area.sqrt())
            }
            (None, Some(h)) => Size::new(self.target_area / h, h),
            (Some(w), None) => Size::new(w, self.target_area / w),
            (Some(w), Some(h)) => {
                if self.use_width {
                    Size::new(w, self.target_area / w)
                } else {
                    Size::new(self.target_area / h, h)
                }
            }
        }
    }
}

pub trait Widget: LayoutSolver + Any {}

impl<T> Widget for T where T: LayoutSolver + Any {}

struct LayoutDemo {
    tree: Rectree,
    world: World,
}

impl LayoutDemo {
    fn new() -> Self {
        Self {
            tree: Rectree::new(),
            world: World::new(),
        }
    }

    fn add_widget<W>(
        &mut self,
        parent: Option<NodeId>,
        color: Color,
        add_content: impl FnOnce(&mut Self, NodeId) -> W,
    ) -> NodeId
    where
        W: Widget + 'static,
    {
        let mut node = RectNode::new();
        if let Some(parent) = parent {
            node = node.with_parent(parent);
        }
        let id = self.tree.insert(node);

        let w = Box::new(add_content(self, id));
        self.world.widgets.insert(id, w);
        self.world.node_colors.insert(id, color);

        id
    }

    fn draw_tree(&self, scene: &mut Scene, transform: Affine) {
        // Start traversal from the root IDs provided by the tree.
        for root_id in self.tree.root_ids() {
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                // Get node from tree.
                let node = self.tree.get(&node_id);

                // Get world_translation.
                let world_pos = node.world_translation();

                // Reconstruct rect from world pos and size.
                let world_rect = Rect::from_origin_size(
                    world_pos.to_point(),
                    node.size(),
                );

                // Fetch node color.
                let color = self
                    .world
                    .node_colors
                    .get(&node_id)
                    .cloned()
                    .unwrap_or(Color::WHITE);

                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    color,
                    None,
                    &world_rect,
                );

                scene.stroke(
                    &Stroke::new(2.0),
                    transform,
                    Color::from_rgb8(255, 255, 255),
                    None,
                    &world_rect,
                );

                // Origin markers.
                let origin = Circle::new(world_rect.origin(), 5.0);

                scene.fill(
                    vello::peniko::Fill::NonZero,
                    transform,
                    Color::from_rgb8(255, 50, 50),
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

impl VelloDemo for LayoutDemo {
    fn window_title(&self) -> &'static str {
        "Layout Showcase"
    }
    fn initial_logical_size(&self) -> (f64, f64) {
        (800.0, 600.0)
    }

    fn rebuild_scene(
        &mut self,
        scene: &mut Scene,
        _scale_factor: f64,
    ) {
        // Create an oscillating translation vector.
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        for (i, (id, widget)) in
            self.world.widgets.iter_mut().enumerate()
        {
            let widget = widget.as_mut() as &mut dyn Any;
            if let Some(area) = widget.downcast_mut::<FixedArea>() {
                let time = time + i as f64;
                let oscillation = (time.cos() + 1.0) * AREA;
                area.target_area = AREA + oscillation;
                self.tree.schedule_relayout(*id);
            }
        }

        // Perform layouting.
        self.tree.layout(&self.world);

        self.draw_tree(scene, Affine::IDENTITY);
    }
}

pub trait SplatExt {
    fn splat(v: f64) -> Self;
}

impl SplatExt for Size {
    fn splat(v: f64) -> Self {
        Size::new(v, v)
    }
}

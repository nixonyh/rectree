use hashbrown::HashMap;
use rectree::node::RectNode;
use rectree::{NodeId, Rectree};
use std::num::NonZeroUsize;
use std::sync::Arc;
use vello::kurbo::{Affine, Circle, Rect, Stroke, Vec2};
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::{
    AaConfig, RenderParams, Renderer, RendererOptions, Scene,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;

struct TranslationDemo {
    tree: Rectree,
    root_id: NodeId,
    node_colors: HashMap<NodeId, Color>,
}

impl TranslationDemo {
    fn new() -> Self {
        let mut tree = Rectree::new();
        let mut node_colors = HashMap::new();

        // Insert root node (Blue).
        let root_id = tree.insert_node(RectNode::from_rect(
            Rect::new(0.0, 0.0, 200.0, 200.0),
        ));
        node_colors.insert(root_id, Color::from_rgb8(80, 120, 200));

        // Insert child node (Green) translated relative to root.
        let child_id = tree.insert_node(
            RectNode::from_rect(Rect::new(0.0, 0.0, 80.0, 80.0))
                .with_parent(root_id)
                .with_translation(Vec2::new(40.0, 40.0)),
        );
        node_colors.insert(child_id, Color::from_rgb8(120, 200, 120));

        // Insert grandchild node (Orange) translated relative to the child.
        let grandchild_id = tree.insert_node(
            RectNode::from_rect(Rect::new(0.0, 0.0, 30.0, 30.0))
                .with_parent(child_id)
                .with_translation(Vec2::new(10.0, 10.0)),
        );
        node_colors
            .insert(grandchild_id, Color::from_rgb8(200, 120, 80));

        tree.update_translations();

        Self {
            tree,
            root_id,
            node_colors,
        }
    }

    fn draw_tree(&self, scene: &mut Scene, transform: Affine) {
        // Start traversal from the root IDs provided by the tree.
        for root_id in self.tree.root_ids() {
            let mut stack = vec![*root_id];

            while let Some(node_id) = stack.pop() {
                // Get node from tree.
                if let Some(node) = self.tree.get_node(&node_id) {
                    // Get world_translation.
                    let world_pos = node.world_translation();

                    // Reconstruct rect from world pos and size.
                    let world_rect = Rect::from_origin_size(
                        world_pos.to_point(),
                        *node.size,
                    );

                    // Fetch node color.
                    let color = self
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
                    let origin =
                        Circle::new(world_rect.origin(), 5.0);

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
}

impl VelloDemo for TranslationDemo {
    fn window_title(&self) -> &'static str {
        "Rectree Translation Showcase"
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
        let oscillation =
            Vec2::new(time.cos() * 60.0, time.sin() * 60.0);

        // Modify ONLY the parent's local translation.
        self.tree.with_node_mut(&self.root_id, |node| {
            *node.local_translation = oscillation;
        });

        // Recalculate world positions.
        self.tree.update_translations();

        let transform = Affine::translate((150.0, 150.0));
        self.draw_tree(scene, transform);
    }
}

pub trait VelloDemo {
    fn window_title(&self) -> &'static str;
    fn initial_logical_size(&self) -> (f64, f64);
    fn rebuild_scene(&mut self, scene: &mut Scene, scale_factor: f64);
}

pub struct VelloWinitApp<'s, D: VelloDemo> {
    pub context: RenderContext,
    pub renderer: Option<Renderer>,
    pub state: RenderState<'s>,
    pub scene: Scene,
    pub demo: D,
}

pub enum RenderState<'s> {
    Suspended(Option<Arc<Window>>),
    Active {
        surface: Box<RenderSurface<'s>>,
        window: Arc<Window>,
    },
}

impl<'s, D: VelloDemo> VelloWinitApp<'s, D> {
    pub fn new(demo: D) -> Self {
        Self {
            context: RenderContext::new(),
            renderer: None,
            state: RenderState::Suspended(None),
            scene: Scene::new(),
            demo,
        }
    }
}

impl<D: VelloDemo> ApplicationHandler for VelloWinitApp<'_, D> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let RenderState::Suspended(cached_window) = &mut self.state
        else {
            return;
        };

        let window = cached_window.take().unwrap_or_else(|| {
            let (w, h) = self.demo.initial_logical_size();
            let attr = Window::default_attributes()
                .with_inner_size(LogicalSize::new(w, h))
                .with_title(self.demo.window_title());
            Arc::new(event_loop.create_window(attr).unwrap())
        });

        let size = window.inner_size();
        let surface_future = self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        );
        let surface =
            pollster::block_on(surface_future).expect("surface");

        let device_handle = &self.context.devices[surface.dev_id];
        surface
            .surface
            .configure(&device_handle.device, &surface.config);

        if self.renderer.is_none() {
            self.renderer = Some(
                Renderer::new(
                    &device_handle.device,
                    RendererOptions {
                        use_cpu: false,
                        antialiasing_support:
                            vello::AaSupport::area_only(),
                        num_init_threads: NonZeroUsize::new(1),
                        pipeline_cache: None,
                    },
                )
                .unwrap(),
            );
        }

        self.state = RenderState::Active {
            surface: Box::new(surface),
            window,
        };
    }

    fn suspended(&mut self, _el: &ActiveEventLoop) {
        if let RenderState::Active { window, .. } = &self.state {
            self.state = RenderState::Suspended(Some(window.clone()));
        }
    }

    fn window_event(
        &mut self,
        el: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let (surface, window) = match &mut self.state {
            RenderState::Active { surface, window } => {
                (surface, window)
            }
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => self
                .context
                .resize_surface(surface, s.width, s.height),
            WindowEvent::RedrawRequested => {
                self.scene.reset();
                self.demo.rebuild_scene(
                    &mut self.scene,
                    window.scale_factor(),
                );

                let dev = &self.context.devices[surface.dev_id];
                let texture =
                    surface.surface.get_current_texture().unwrap();

                self.renderer
                    .as_mut()
                    .unwrap()
                    .render_to_texture(
                        &dev.device,
                        &dev.queue,
                        &self.scene,
                        &surface.target_view,
                        &RenderParams {
                            base_color: Color::from_rgb8(20, 20, 30),
                            width: surface.config.width,
                            height: surface.config.height,
                            antialiasing_method: AaConfig::Area,
                        },
                    )
                    .unwrap();

                let mut enc = dev.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None },
                );

                surface.blitter.copy(
                    &dev.device,
                    &mut enc,
                    &surface.target_view,
                    &texture.texture.create_view(
                        &wgpu::TextureViewDescriptor::default(),
                    ),
                );

                dev.queue.submit([enc.finish()]);
                texture.present();
                window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = VelloWinitApp::new(TranslationDemo::new());

    event_loop.run_app(&mut app).unwrap();
}

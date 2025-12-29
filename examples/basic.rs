use kurbo::Size;
use rectree::kurbo::Vec2;
use rectree::node::RectNode;
use rectree::{Constraint, NodeId, Rectree};

fn main() {
    let mut tree = Rectree::new();

    // Insert root rect.
    let root_id =
        tree.insert_node(RectNode::from_size(Size::splat(100.0)));
    let child_id = tree.insert_node(
        RectNode::from_translation_size(
            Vec2::splat(10.0),
            Size::splat(10.0),
        )
        .with_parent(root_id),
    );
    tree.insert_node(
        RectNode::from_translation_size(
            Vec2::splat(10.0),
            Size::splat(10.0),
        )
        .with_parent(child_id),
    );

    println!("Before update...");
    print_tree(&tree);
    tree.update_translations();
    println!("After update...");
    print_tree(&tree);

    println!(
        "\nPerform translation tweaks on node #[{}]...",
        child_id.index()
    );
    tree.with_node_mut(&child_id, |node| {
        *node.local_translation += Vec2::splat(3.0);
    });

    println!("\nBefore update...");
    print_tree(&tree);
    tree.update_translations();
    println!("After update...");
    print_tree(&tree);
}

fn print_tree(tree: &Rectree) {
    let mut stack = tree.root_ids().iter().collect::<Vec<_>>();

    while let Some(id) = stack.pop() {
        if let Some(node) = tree.get_node(id) {
            for _ in 0..node.depth() {
                print!("  ");
            }

            println!(
                "translation: {}, size: {} #[{}]",
                node.world_translation(),
                &*node.size,
                id.index()
            );

            stack.extend(node.children().iter());
        }
    }
}

struct VerticalCenteredList {
    padding: f64,
    // In real world scenario, this should just store the ids
    // mapping to an arena of widgets.
    children: Vec<(NodeId, Box<dyn Layouter>)>,
}

impl Layouter for VerticalCenteredList {
    fn layout(
        &self,
        constraint: Constraint,
        tree: &mut Rectree,
    ) -> Size {
        let max_width = constraint.width.map_or_else(
            || {
                let mut max_width = 0.0;

                for (child_id, _) in self.children.iter() {
                    if let Some(node) = tree.get_node(child_id) {
                        max_width = node.size.width.max(max_width);
                    }
                }

                max_width
            },
            |width| width,
        );

        let mut y = self.padding;
        for (child_id, frame) in self.children.iter() {
            let child_size = frame.layout(
                Constraint {
                    width: Some(max_width),
                    ..Default::default()
                },
                tree,
            );
            tree.with_node_mut(child_id, |node| {
                *node.size = child_size;
                let remainder = max_width - node.size.width;
                node.local_translation.x = remainder * 0.5;
                node.local_translation.y = y;
                y += node.size.height + self.padding;
            });
        }

        Size::new(max_width, y)
    }
}

struct FixedArea {
    /// Use width if constrained on both axis.
    /// (Acts like a text widget.)
    pub use_width: bool,
    pub target_area: f64,
}

impl Layouter for FixedArea {
    fn layout(
        &self,
        constraint: Constraint,
        _tree: &mut Rectree,
    ) -> Size {
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

pub trait Layouter {
    fn layout(
        &self,
        constraint: Constraint,
        tree: &mut Rectree,
    ) -> Size;
}

pub trait Builder {
    fn build(&self, constraint: Constraint) -> Vec2;
    fn position(&self, children: &mut [RectNode]);
}

pub trait SplatExt {
    fn splat(v: f64) -> Self;
}

impl SplatExt for Size {
    fn splat(v: f64) -> Self {
        Size::new(v, v)
    }
}

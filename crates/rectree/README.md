# Rectree

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/rectree#license)
[![Crates.io](https://img.shields.io/crates/v/rectree.svg)](https://crates.io/crates/rectree)
[![Downloads](https://img.shields.io/crates/d/rectree.svg)](https://crates.io/crates/rectree)
[![Docs](https://docs.rs/rectree/badge.svg)](https://docs.rs/rectree/latest/rectree/)
[![CI](https://github.com/voxell-tech/rectree/workflows/CI/badge.svg)](https://github.com/voxell-tech/rectree/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

**Rectree** proposes a simple concept towards user interfaces, that
everything can be represented as a tree of axis-aligned bounding
boxes (AABB). In **Rectree**, these are represented as rectangles,
hence the name "rect-tree".

Rectree is designed to be:
- **Deterministic**: identical inputs always produce identical
  layouts.
- **Incremental**: only affected subtrees are recomputed.
- **Policy-free**: layout behavior is defined by user-provided
  algorithms.

![layout_basic](/.github/assets/layout_basic.png)

## Core Concepts

| Type / Trait    | Role                                           |
| --------------- | ---------------------------------------------- |
| `Rectree`       | tree structure and per-node layout logic       |
| `RectNodes`     | flat mutable storage for per-node numbers      |
| `RectContext`   | restricted build-time view of `RectNodes`      |
| `RectNode`      | per-node data (size, constraint, translation)  |
| `NodeState`     | bitflags that short-circuit incremental passes |
| `Constraint`    | min/max size bounds, flowing top-down          |
| `Size`          | resolved dimensions, flowing bottom-up         |

Rectree itself does not impose a specific layout style (e.g. flexbox,
grid). Instead, it provides a strict data-flow model on top of which
layout algorithms can be built.

## Three-Pass Layout

Each call to `layout` runs three passes in order:

1. **constrain** (top-down): each node derives the constraint it
   passes to its children via `Rectree::constrain`.
2. **build** (bottom-up): each node measures itself given its
   constraint and child sizes via `Rectree::build`.
3. **propagate_translation** (top-down): local translations set
   during build are accumulated into world-space positions.

Each pass is short-circuited by `NodeState` flags so only nodes
that actually changed are reprocessed.

### Layout Rules

1. Constraints flow strictly top-down (parent to child).
2. Sizes flow strictly bottom-up (child to parent).
3. A parent may pass a different constraint to each child.
4. Given the same constraint, an unmodified node must always
   produce the same size.
5. The build pass must not write child sizes, only child
   translations. This is enforced by `RectContext`.

## Example

```rust
use std::collections::HashMap;
use rectree::{
    Constraint, RectContext, RectNode, RectNodes,
    Rectree, Size, layout,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Id(u32);

// Flat node storage backed by a HashMap.
struct Store(HashMap<Id, RectNode<Id>>);

impl RectNodes for Store {
    type Id = Id;

    fn get_node(&self, id: &Id) -> Option<&RectNode<Id>> {
        self.0.get(id)
    }

    fn get_node_mut(&mut self, id: &Id)
        -> Option<&mut RectNode<Id>>
    {
        self.0.get_mut(id)
    }
}

// Tree with one root and one child. The root fills its
// constraint; the child has a fixed 100x50 size.
struct Tree { root: Id, child: Id }

impl Rectree for Tree {
    type Id = Id;

    fn children(&self, id: &Id)
        -> impl IntoIterator<Item = &Id>
    {
        if *id == self.root { Some(&self.child) } else { None }
    }

    // Pass the parent constraint to children unchanged.
    fn constrain(&self, _: &Id, parent: Constraint)
        -> Constraint
    {
        parent
    }

    fn build<N: RectContext<Id = Id>>(
        &self,
        id: &Id,
        constraint: Constraint,
        _nodes: &mut N,
    ) -> Size {
        if *id == self.child {
            Size::new(100.0, 50.0)
        } else {
            constraint.max
        }
    }
}

let root = Id(0);
let child = Id(1);
let tree = Tree { root, child };

let mut store = Store(HashMap::new());
store.0.insert(root, RectNode::new(None));
store.0.insert(child, RectNode::new(Some(root)));

// Constrain the root to an 800x600 window.
store.0.get_mut(&root).unwrap().constraint =
    Constraint::tight(Size::new(800.0, 600.0));

layout(&tree, &mut store, &root);

// Child is placed at the origin by default.
assert_eq!(store.0[&child].world_translation.x, 0.0);
assert_eq!(store.0[&child].world_translation.y, 0.0);
assert_eq!(store.0[&child].size, Size::new(100.0, 50.0));
```

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`rectree` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.

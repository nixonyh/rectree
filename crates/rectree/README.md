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

## Core Concepts

- `Rectree`: a hierarchical tree of rectangular nodes.
- `Constraint`: size limitations flowing *from parent to child*.
- `Size`: resolved dimensions flowing *from child to parent*.
- `LayoutSolver`: user-defined logic that computes constraints,
  sizes, and relative translations.

Rectree itself does not impose a specific layout style (e.g. flexbox,
grid). Instead, it provides a strict data-flow model on top of which
layout algorithms can be built.

### Layout Rules

1. The only data that can flow down the tree is `Constraint`.
2. The only data that can flow up the tree is `Size`.
3. Each child, no matter the order, will recieve the same `Constraint`
   from the parent.
4. Given the same `Constraint`, an unmodified node must always
   produce the same `Size`.

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`rectree` is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.


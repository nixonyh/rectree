#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use core::ops::Deref;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use kurbo::{Point, Rect};

use crate::morton::{MortonCode, find_split, morton_2d_f64};

pub use kurbo;

pub mod morton;

/// **Spatree** implements a Linear Bounding Volume Hierarchy (LBVH).
///
/// It uses _Morton encoding_ to map 2D spaital coordinates onto a 1D
/// Z-order curve. Sorting these codes ensures spatially close objects
/// are adjacent in memory, allowing for efficient top-down hierarchy
/// generation.
#[derive(Default)]
pub struct Spatree {
    global_bound: Rect,
    rects: Vec<Rect>,
    nodes: Vec<Node>,
}

// Builders.
impl Spatree {
    /// Creates a new empty [`Spatree`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new [`Rect`] into the spatial tree.
    ///
    /// If this is performed after [`Self::build()`], a rebuild will
    /// be required to cater for the change!
    pub fn push_rect(&mut self, rect: Rect) -> RectId {
        let index = self.rects.len();
        self.rects.push(rect);
        // Fit the global bound to the new rect.
        self.global_bound = self.global_bound.union(rect);
        RectId(index)
    }

    /// Get a specific [`Rect`] for a given [`RectId`].
    pub fn get_rect(&self, id: RectId) -> Option<&Rect> {
        self.rects.get(*id)
    }

    /// Obtain the global bounding box of the spatial tree.
    /// Thi global bound is accumulated during
    /// [`Self::push_rect()`] calls.
    pub fn global_bound(&self) -> &Rect {
        &self.global_bound
    }

    /// Constructs a spatial hierarchy (LBVH) from the current set of rectangles.
    ///
    /// ### Arguments
    ///
    /// - `point_from_rect`: A closure that determines the stable
    ///   representative point (e.g., center or top-left) of a `Rect`
    ///   used for Morton encoding.
    ///
    /// After construction, all internal node bounding boxes are computed.
    ///
    /// If [`Self::global_bound()`] has zero area, the tree is left
    /// empty since no meaningful spatial ordering can be derived.
    pub fn build<F>(&mut self, point_from_rect: F)
    where
        F: Fn(&Rect) -> Point,
    {
        let bound_size = self.global_bound.size();
        // There is point in building a spatial tree when there is no
        // space within the max bound.
        if bound_size.is_zero_area() {
            return;
        }

        let mut morton_codes = self
            .rects
            .iter()
            .enumerate()
            .map(|(index, rect)| {
                let point = point_from_rect(rect);
                let x = point.x / bound_size.width;
                let y = point.y / bound_size.height;

                let code = morton_2d_f64(x, y);
                MortonCode { code, index }
            })
            .collect::<Box<_>>();

        morton_codes.sort_unstable();

        // Build internal nodes.
        self.nodes = generate_hierarchy(&morton_codes);
        self.calculate_internal_bounds();
    }

    /// Calculate the bounds of all the internal nodes.
    fn calculate_internal_bounds(&mut self) {
        if self.nodes.is_empty() {
            return;
        }

        // Because internal nodes were allocated top-down, children
        // always have a higher index than their parents. By iterating
        // backwards, we process the tree bottom-up.
        for i in (0..self.nodes.len()).rev() {
            let mut combined_rect = None;

            // Check both children to compute the unioned bounding box
            for child_id in self.nodes[i].children {
                let child_rect = match child_id {
                    NodeId::Leaf(rect_id) => {
                        // Leaf bounds are already known from the input rects
                        self.rects[rect_id]
                    }
                    NodeId::Internal(idx) => {
                        // Because idx > i, this child's rect was
                        // already calculated in a previous iteration of this loop.
                        self.nodes[idx].rect
                    }
                    NodeId::Invalid => Rect::ZERO,
                };

                // Union the child's rect into the parent's rect
                combined_rect = Some(match combined_rect {
                    None => child_rect,
                    Some(existing) => child_rect.union(existing),
                });
            }

            if let Some(final_rect) = combined_rect {
                self.nodes[i].rect = final_rect;
            }
        }
    }
}

/// Queries.
impl Spatree {
    /// Query for all hits for an arbitrary target.
    pub fn query<T, F>(
        &self,
        target: T,
        hit_condition: F,
    ) -> Vec<RectId>
    where
        F: Fn(&Rect, &T) -> bool,
    {
        let mut hits = Vec::new();

        if self.nodes.is_empty() {
            // There's no tree, if there's just one rect, do a hit
            // test for it.
            if let Some(rect) = self.rects.first()
                && hit_condition(rect, &target)
            {
                hits.push(RectId(0));
            }
        } else {
            // Traverse the tree.
            let mut stack = vec![0];

            while let Some(node_idx) = stack.pop() {
                let node = self.nodes[node_idx];

                // Skip the tree if it's not a hit.
                if !hit_condition(&node.rect, &target) {
                    continue;
                }

                for child in node.children.iter() {
                    match child {
                        NodeId::Internal(child_idx) => {
                            stack.push(*child_idx)
                        }
                        NodeId::Leaf(leaf_idx) => {
                            if hit_condition(
                                &self.rects[*leaf_idx],
                                &target,
                            ) {
                                hits.push(RectId(*leaf_idx));
                            }
                        }
                        NodeId::Invalid => continue,
                    }
                }
            }
        }

        hits
    }

    /// Query for a singles hit for an arbitrary target.
    pub fn query_single<T, H, C>(
        &self,
        target: T,
        hit_condition: H,
        conflict_resolution: C,
    ) -> Option<RectId>
    where
        H: Fn(&Rect, &T) -> bool,
        C: Fn(RectId, RectId) -> RectId,
    {
        let mut hit = None;
        // let mut hits = Vec::new();

        if self.nodes.is_empty() {
            // There's no tree, if there's just one rect, do a hit
            // test for it.
            if let Some(rect) = self.rects.first()
                && hit_condition(rect, &target)
            {
                hit = Some(RectId(0));
            }
        } else {
            // Traverse the tree.
            let mut stack = vec![0];

            while let Some(node_idx) = stack.pop() {
                let node = self.nodes[node_idx];

                // Skip the tree if it's not a hit.
                if !hit_condition(&node.rect, &target) {
                    continue;
                }

                for child in node.children.iter() {
                    match child {
                        NodeId::Internal(child_idx) => {
                            stack.push(*child_idx)
                        }
                        NodeId::Leaf(leaf_idx) => {
                            if hit_condition(
                                &self.rects[*leaf_idx],
                                &target,
                            ) {
                                let new_hit = RectId(*leaf_idx);
                                match &mut hit {
                                    Some(hit) => {
                                        *hit = conflict_resolution(
                                            *hit, new_hit,
                                        );
                                    }
                                    None => hit = Some(new_hit),
                                }
                            }
                        }
                        NodeId::Invalid => continue,
                    }
                }
            }
        }

        hit
    }

    /// Query for all rects that contains the given [`Point`].
    pub fn query_point(&self, point: Point) -> Vec<RectId> {
        self.query(
            point,
            #[inline(always)]
            |rect, point| rect.contains(*point),
        )
    }

    /// Query for all rects that overlaps the given [`Rect`].
    pub fn query_rect(&self, rect: Rect) -> Vec<RectId> {
        self.query(
            rect,
            #[inline(always)]
            |rect, target_rect| rect.overlaps(*target_rect),
        )
    }

    /// Query for a single rects that contains the given [`Point`].
    pub fn query_point_single<C>(
        &self,
        point: Point,
        conflict_resolution: C,
    ) -> Option<RectId>
    where
        C: Fn(RectId, RectId) -> RectId,
    {
        self.query_single(
            point,
            #[inline(always)]
            |rect, point| rect.contains(*point),
            conflict_resolution,
        )
    }

    /// Query for a single rects that contains the given [`Point`].
    pub fn query_rect_single<C>(
        &self,
        rect: Rect,
        conflict_resolution: C,
    ) -> Option<RectId>
    where
        C: Fn(RectId, RectId) -> RectId,
    {
        self.query_single(
            rect,
            #[inline(always)]
            |rect, target_rect| rect.overlaps(*target_rect),
            conflict_resolution,
        )
    }
}

/// An internal node within the [`Spatree`].
#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub rect: Rect,
    pub parent: Option<usize>,
    pub children: [NodeId; 2],
}

impl Node {
    /// Empty node with zero area, no children, and no parent.
    pub const EMPTY: Self = Self {
        rect: Rect::ZERO,
        parent: None,
        children: [NodeId::Invalid; 2],
    };
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum NodeId {
    Internal(usize),
    Leaf(usize),
    Invalid,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct RectId(usize);

impl RectId {
    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl Deref for RectId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Top down hierarchy building for single threaded algorithm.
pub fn generate_hierarchy(codes: &[MortonCode]) -> Vec<Node> {
    let len = codes.len();
    if len <= 1 {
        return Vec::new();
    }

    // A binary tree with N leaves has exactly N - 1 internal nodes.
    let mut internal_nodes = vec![Node::EMPTY; len - 1];

    /// Represents a range to be split and its connection to the tree.
    struct BuildStack {
        first: usize,
        last: usize,
        parent_idx: Option<usize>,
        /// `0` for left, `1` for right.
        child_slot: usize,
    }

    let mut stack = Vec::with_capacity(len);
    let mut node_idx = 0;

    // First build stakc will have the full range.
    stack.push(BuildStack {
        first: 0,
        last: internal_nodes.len(),
        parent_idx: None,
        child_slot: 0,
    });

    while let Some(task) = stack.pop() {
        let BuildStack {
            first,
            last,
            parent_idx,
            child_slot,
        } = task;

        let curr_node_id = if first == last {
            // Single element range represents a leaf node.
            NodeId::Leaf(codes[first].index)
        } else {
            // Internal node case.
            let node_id = NodeId::Internal(node_idx);
            let split = find_split(codes, first, last);

            // Push right sub-range then left sub-range (LIFO).
            stack.push(BuildStack {
                first,
                last: split,
                parent_idx: Some(node_idx),
                child_slot: 0,
            });
            stack.push(BuildStack {
                first: split + 1,
                last,
                parent_idx: Some(node_idx),
                child_slot: 1,
            });

            node_idx += 1;
            node_id
        };

        // Link the current node to its parent if it's not the root.
        if let Some(parent_idx) = parent_idx {
            internal_nodes[parent_idx].children[child_slot] =
                curr_node_id;

            // If the current node is internal, set its parent index.
            if let NodeId::Internal(curr_idx) = curr_node_id {
                internal_nodes[curr_idx].parent = Some(parent_idx);
            }
        }
    }

    internal_nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let mut tree = Spatree::new();
        tree.build(|r| r.center());

        assert!(tree.rects.is_empty());
        assert!(tree.nodes.is_empty());
        assert_eq!(tree.global_bound().area(), 0.0);

        let hits = tree.query_point(Point::new(10.0, 10.0));
        assert!(hits.is_empty());
    }

    #[test]
    fn test_single_item_tree() {
        let mut tree = Spatree::new();
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let id = tree.push_rect(r1);

        tree.build(|r| r.center());

        // Single item means N-1 = 0 internal nodes.
        assert!(tree.nodes.is_empty());

        let hits = tree.query_point(Point::new(5.0, 5.0));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], id);
    }

    #[test]
    fn test_hierarchy_structure_and_bounds() {
        let mut tree = Spatree::new();

        // 4 corners of a 100x100 area.
        // Top left.
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        // Top right.
        let r2 = Rect::new(90.0, 0.0, 100.0, 10.0);
        // Bottom left.
        let r3 = Rect::new(0.0, 90.0, 10.0, 100.0);
        // Bottom right.
        let r4 = Rect::new(90.0, 90.0, 100.0, 100.0);

        tree.push_rect(r1);
        tree.push_rect(r2);
        tree.push_rect(r3);
        tree.push_rect(r4);

        tree.build(|r| r.center());

        // N items = N-1 internal nodes.
        assert_eq!(tree.nodes.len(), 3);

        // Root is the first node generated in top-down.
        let root = &tree.nodes[0];
        let expected_union = r1.union(r2).union(r3).union(r4);

        assert_eq!(root.rect.x0, expected_union.x0);
        assert_eq!(root.rect.y0, expected_union.y0);
        assert_eq!(root.rect.x1, expected_union.x1);
        assert_eq!(root.rect.y1, expected_union.y1);
    }

    #[test]
    fn test_query_point() {
        let mut tree = Spatree::new();
        let r1 = Rect::new(10.0, 10.0, 30.0, 30.0);
        let r2 = Rect::new(20.0, 20.0, 40.0, 40.0);

        let id1 = tree.push_rect(r1);
        let id2 = tree.push_rect(r2);

        tree.build(|r| r.center());

        // Point inside intersection.
        let hits = tree.query_point(Point::new(25.0, 25.0));
        assert_eq!(hits.len(), 2);
        assert!(hits.contains(&id1));
        assert!(hits.contains(&id2));
    }

    #[test]
    fn test_query_rect() {
        let mut tree = Spatree::new();

        // Define 3 distinct areas.
        // Top left.
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        // Top right.
        let r2 = Rect::new(20.0, 0.0, 30.0, 10.0);
        // Bottom wide strip.
        let r3 = Rect::new(0.0, 20.0, 30.0, 30.0);

        let id1 = tree.push_rect(r1);
        let id2 = tree.push_rect(r2);
        let id3 = tree.push_rect(r3);

        tree.build(|r| r.center());

        // 1. Overlaps only `r1`.
        let q1 = Rect::new(-5.0, -5.0, 5.0, 5.0);
        let hits = tree.query_rect(q1);
        assert_eq!(hits.len(), 1);
        assert!(hits.contains(&id1));

        // 2. Overlaps `r1` and `r2` but not `r3`.
        let q2 = Rect::new(5.0, 2.0, 25.0, 8.0);
        let hits = tree.query_rect(q2);
        assert_eq!(hits.len(), 2);
        assert!(hits.contains(&id1));
        assert!(hits.contains(&id2));
        assert!(!hits.contains(&id3));

        // 3. Overlaps all 3.
        let q3 = Rect::new(5.0, 5.0, 25.0, 25.0);
        let hits = tree.query_rect(q3);
        assert_eq!(hits.len(), 3);
        assert!(hits.contains(&id1));
        assert!(hits.contains(&id2));
        assert!(hits.contains(&id3));

        // 4. Complete miss
        let q4 = Rect::new(100.0, 100.0, 110.0, 110.0);
        let hits = tree.query_rect(q4);
        assert!(hits.is_empty());
    }

    /// Largest index win (simulating a stack/z-order).
    #[inline(always)]
    fn stack_conflict_resolution(a: RectId, b: RectId) -> RectId {
        if a > b { a } else { b }
    }

    #[test]
    fn test_query_point_single() {
        let mut tree = Spatree::new();

        // Largest (lowest).
        let id0 = tree.push_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
        // Middle (center).
        let id1 = tree.push_rect(Rect::new(0.0, 0.0, 50.0, 50.0));
        // Smallest (top).
        let id2 = tree.push_rect(Rect::new(0.0, 0.0, 10.0, 10.0));

        assert!(id0 < id1 && id1 < id2, "Ids should be incremented!");

        tree.build(|r| r.center());

        // 1. Point hits all 3.
        let p1 = Point::new(5.0, 5.0);
        let hit =
            tree.query_point_single(p1, stack_conflict_resolution);
        assert_eq!(hit, Some(id2));

        // 2. Point hits `id0` and `id1`, but misses the tiny `id2`.
        let p2 = Point::new(20.0, 20.0);
        let hit =
            tree.query_point_single(p2, stack_conflict_resolution);
        assert_eq!(hit, Some(id1));

        // 3. Point hits only the large base `id0`.
        let p3 = Point::new(75.0, 75.0);
        let hit =
            tree.query_point_single(p3, stack_conflict_resolution);
        assert_eq!(hit, Some(id0));

        // 4. Complete miss.
        let p4 = Point::new(150.0, 150.0);
        let hit =
            tree.query_point_single(p4, stack_conflict_resolution);
        assert!(hit.is_none());
    }

    #[test]
    fn test_query_rect_single() {
        let mut tree = Spatree::new();

        // Largest (lowest).
        let id0 = tree.push_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
        // Middle (center).
        let id1 = tree.push_rect(Rect::new(0.0, 0.0, 50.0, 50.0));
        // Smallest (top).
        let id2 = tree.push_rect(Rect::new(0.0, 0.0, 10.0, 10.0));

        tree.build(|r| r.center());

        // 1. Overlaps all 3.
        let q1 = Rect::new(2.0, 2.0, 8.0, 8.0);
        let hit =
            tree.query_rect_single(q1, stack_conflict_resolution);
        assert_eq!(hit, Some(id2));

        // 2. Overlaps `id0` and `id1`, but misses `id2`.
        let q2 = Rect::new(15.0, 15.0, 25.0, 25.0);
        let hit =
            tree.query_rect_single(q2, stack_conflict_resolution);
        assert_eq!(hit, Some(id1));

        // 3. Overlaps only the largest base `id0`.
        let q3 = Rect::new(60.0, 60.0, 70.0, 70.0);
        let hit =
            tree.query_rect_single(q3, stack_conflict_resolution);
        assert_eq!(hit, Some(id0));

        // 4. Complete miss.
        let q4 = Rect::new(200.0, 200.0, 210.0, 210.0);
        let hit =
            tree.query_rect_single(q4, stack_conflict_resolution);
        assert!(hit.is_none());
    }
}

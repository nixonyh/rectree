/// A 2D size in resolved pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    /// Zero size on both axes.
    pub const ZERO: Self = Self::splat(0.0);

    /// Infinite size on both axes.
    pub const INFINITY: Self = Self::splat(f32::INFINITY);

    /// Creates a `Size` from explicit `width` and `height`.
    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Creates a `Size` with the same value on both axes.
    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }
}

/// A 2D position or translation vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// Zero vector on both axes.
    pub const ZERO: Self = Self::splat(0.0);

    /// Creates a `Vec2` from explicit `x` and `y`.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Creates a `Vec2` with the same value on both axes.
    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }
}

impl core::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

/// Min/max size bounds passed top-down through the layout tree.
///
/// A `Constraint` tells a node how large it is allowed to be.
/// The node measures itself within these bounds and returns a
/// [`Size`].
///
/// # Axis independence
///
/// Each axis is constrained independently. Setting `max` to
/// [`f32::INFINITY`] leaves that axis unconstrained (see
/// [`Self::unbounded`], [`Self::fixed_width`],
/// [`Self::fixed_height`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraint {
    pub min: Size,
    pub max: Size,
}

impl Constraint {
    /// The node must be exactly `size`, with no flexibility.
    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    /// The node may choose any size from zero up to `max`.
    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::ZERO,
            max,
        }
    }

    /// No bounds on either axis; the node may be any size.
    ///
    /// This is the [`Default`] value for `Constraint`, used for
    /// root nodes that have no parent imposing bounds.
    pub const fn unbounded() -> Self {
        Self {
            min: Size::ZERO,
            max: Size::INFINITY,
        }
    }

    /// Bounded width, unbounded height.
    ///
    /// Use this for vertical scroll containers.
    pub const fn fixed_width(width: f32) -> Self {
        Self {
            min: Size::ZERO,
            max: Size {
                width,
                height: f32::INFINITY,
            },
        }
    }

    /// Bounded height, unbounded width.
    ///
    /// Use this for horizontal scroll containers.
    pub const fn fixed_height(height: f32) -> Self {
        Self {
            min: Size::ZERO,
            max: Size {
                width: f32::INFINITY,
                height,
            },
        }
    }

    /// Clamps `size` so it satisfies this constraint.
    ///
    /// Each axis is clamped independently to `[min, max]`. Use
    /// this at the end of a [`crate::Rectree::build`]
    /// implementation to ensure the returned [`Size`] respects
    /// the constraint.
    pub const fn constrain(&self, size: Size) -> Size {
        Size {
            width: size.width.max(self.min.width).min(self.max.width),
            height: size
                .height
                .max(self.min.height)
                .min(self.max.height),
        }
    }
}

impl Default for Constraint {
    /// Returns [`Self::unbounded`], the default for root nodes
    /// that have no parent imposing bounds.
    fn default() -> Self {
        Self::unbounded()
    }
}

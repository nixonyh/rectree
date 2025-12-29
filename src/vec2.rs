//! A 2D vector representation. Implementation is mostly carried over from `glam`.

use core::fmt;
use core::ops::*;

#[cfg(not(feature = "double-precision"))]
pub type Scalar = f32;
#[cfg(feature = "double-precision")]
pub type Scalar = f64;

#[derive(Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Vec2 {
    pub x: Scalar,
    pub y: Scalar,
}

impl Vec2 {
    pub fn new(x: Scalar, y: Scalar) -> Self {
        Self { x, y }
    }
}

impl Div for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x.div(rhs.x),
            y: self.y.div(rhs.y),
        }
    }
}

impl Div<&Self> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &Self) -> Self {
        self.div(*rhs)
    }
}

impl Div<&Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: &Vec2) -> Vec2 {
        (*self).div(*rhs)
    }
}

impl Div<Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Vec2) -> Vec2 {
        (*self).div(rhs)
    }
}

impl DivAssign for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x.div_assign(rhs.x);
        self.y.div_assign(rhs.y);
    }
}

impl DivAssign<&Self> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: &Self) {
        self.div_assign(*rhs);
    }
}

impl Div<Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Scalar) -> Self {
        Self {
            x: self.x.div(rhs),
            y: self.y.div(rhs),
        }
    }
}

impl Div<&Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &Scalar) -> Self {
        self.div(*rhs)
    }
}

impl Div<&Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: &Scalar) -> Vec2 {
        (*self).div(*rhs)
    }
}

impl Div<Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Scalar) -> Vec2 {
        (*self).div(rhs)
    }
}

impl DivAssign<Scalar> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: Scalar) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
    }
}

impl DivAssign<&Scalar> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: &Scalar) {
        self.div_assign(*rhs);
    }
}

impl Div<Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.div(rhs.x),
            y: self.div(rhs.y),
        }
    }
}

impl Div<&Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: &Vec2) -> Vec2 {
        self.div(*rhs)
    }
}

impl Div<&Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: &Vec2) -> Vec2 {
        (*self).div(*rhs)
    }
}

impl Div<Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn div(self, rhs: Vec2) -> Vec2 {
        (*self).div(rhs)
    }
}

impl Mul for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x.mul(rhs.x),
            y: self.y.mul(rhs.y),
        }
    }
}

impl Mul<&Self> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: &Self) -> Self {
        self.mul(*rhs)
    }
}

impl Mul<&Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: &Vec2) -> Vec2 {
        (*self).mul(*rhs)
    }
}

impl Mul<Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        (*self).mul(rhs)
    }
}

impl MulAssign for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x.mul_assign(rhs.x);
        self.y.mul_assign(rhs.y);
    }
}

impl MulAssign<&Self> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) {
        self.mul_assign(*rhs);
    }
}

impl Mul<Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Scalar) -> Self {
        Self {
            x: self.x.mul(rhs),
            y: self.y.mul(rhs),
        }
    }
}

impl Mul<&Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: &Scalar) -> Self {
        self.mul(*rhs)
    }
}

impl Mul<&Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: &Scalar) -> Vec2 {
        (*self).mul(*rhs)
    }
}

impl Mul<Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Scalar) -> Vec2 {
        (*self).mul(rhs)
    }
}

impl MulAssign<Scalar> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Scalar) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
    }
}

impl MulAssign<&Scalar> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Scalar) {
        self.mul_assign(*rhs);
    }
}

impl Mul<Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.mul(rhs.x),
            y: self.mul(rhs.y),
        }
    }
}

impl Mul<&Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: &Vec2) -> Vec2 {
        self.mul(*rhs)
    }
}

impl Mul<&Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: &Vec2) -> Vec2 {
        (*self).mul(*rhs)
    }
}

impl Mul<Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        (*self).mul(rhs)
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x.add(rhs.x),
            y: self.y.add(rhs.y),
        }
    }
}

impl Add<&Self> for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: &Self) -> Self {
        self.add(*rhs)
    }
}

impl Add<&Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: &Vec2) -> Vec2 {
        (*self).add(*rhs)
    }
}

impl Add<Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        (*self).add(rhs)
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
    }
}

impl AddAssign<&Self> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        self.add_assign(*rhs);
    }
}

impl Add<Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Scalar) -> Self {
        Self {
            x: self.x.add(rhs),
            y: self.y.add(rhs),
        }
    }
}

impl Add<&Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: &Scalar) -> Self {
        self.add(*rhs)
    }
}

impl Add<&Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: &Scalar) -> Vec2 {
        (*self).add(*rhs)
    }
}

impl Add<Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Scalar) -> Vec2 {
        (*self).add(rhs)
    }
}

impl AddAssign<Scalar> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Scalar) {
        self.x.add_assign(rhs);
        self.y.add_assign(rhs);
    }
}

impl AddAssign<&Scalar> for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: &Scalar) {
        self.add_assign(*rhs);
    }
}

impl Add<Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.add(rhs.x),
            y: self.add(rhs.y),
        }
    }
}

impl Add<&Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: &Vec2) -> Vec2 {
        self.add(*rhs)
    }
}

impl Add<&Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: &Vec2) -> Vec2 {
        (*self).add(*rhs)
    }
}

impl Add<Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn add(self, rhs: Vec2) -> Vec2 {
        (*self).add(rhs)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x.sub(rhs.x),
            y: self.y.sub(rhs.y),
        }
    }
}

impl Sub<&Self> for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: &Self) -> Self {
        self.sub(*rhs)
    }
}

impl Sub<&Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: &Vec2) -> Vec2 {
        (*self).sub(*rhs)
    }
}

impl Sub<Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Vec2 {
        (*self).sub(rhs)
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x.sub_assign(rhs.x);
        self.y.sub_assign(rhs.y);
    }
}

impl SubAssign<&Self> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        self.sub_assign(*rhs);
    }
}

impl Sub<Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Scalar) -> Self {
        Self {
            x: self.x.sub(rhs),
            y: self.y.sub(rhs),
        }
    }
}

impl Sub<&Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: &Scalar) -> Self {
        self.sub(*rhs)
    }
}

impl Sub<&Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: &Scalar) -> Vec2 {
        (*self).sub(*rhs)
    }
}

impl Sub<Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Scalar) -> Vec2 {
        (*self).sub(rhs)
    }
}

impl SubAssign<Scalar> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Scalar) {
        self.x.sub_assign(rhs);
        self.y.sub_assign(rhs);
    }
}

impl SubAssign<&Scalar> for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Scalar) {
        self.sub_assign(*rhs);
    }
}

impl Sub<Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.sub(rhs.x),
            y: self.sub(rhs.y),
        }
    }
}

impl Sub<&Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: &Vec2) -> Vec2 {
        self.sub(*rhs)
    }
}

impl Sub<&Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: &Vec2) -> Vec2 {
        (*self).sub(*rhs)
    }
}

impl Sub<Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn sub(self, rhs: Vec2) -> Vec2 {
        (*self).sub(rhs)
    }
}

impl Rem for Vec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: Self) -> Self {
        Self {
            x: self.x.rem(rhs.x),
            y: self.y.rem(rhs.y),
        }
    }
}

impl Rem<&Self> for Vec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: &Self) -> Self {
        self.rem(*rhs)
    }
}

impl Rem<&Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: &Vec2) -> Vec2 {
        (*self).rem(*rhs)
    }
}

impl Rem<Vec2> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: Vec2) -> Vec2 {
        (*self).rem(rhs)
    }
}

impl RemAssign for Vec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: Self) {
        self.x.rem_assign(rhs.x);
        self.y.rem_assign(rhs.y);
    }
}

impl RemAssign<&Self> for Vec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: &Self) {
        self.rem_assign(*rhs);
    }
}

impl Rem<Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: Scalar) -> Self {
        Self {
            x: self.x.rem(rhs),
            y: self.y.rem(rhs),
        }
    }
}

impl Rem<&Scalar> for Vec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: &Scalar) -> Self {
        self.rem(*rhs)
    }
}

impl Rem<&Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: &Scalar) -> Vec2 {
        (*self).rem(*rhs)
    }
}

impl Rem<Scalar> for &Vec2 {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: Scalar) -> Vec2 {
        (*self).rem(rhs)
    }
}

impl RemAssign<Scalar> for Vec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: Scalar) {
        self.x.rem_assign(rhs);
        self.y.rem_assign(rhs);
    }
}

impl RemAssign<&Scalar> for Vec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: &Scalar) {
        self.rem_assign(*rhs);
    }
}

impl Rem<Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.rem(rhs.x),
            y: self.rem(rhs.y),
        }
    }
}

impl Rem<&Vec2> for Scalar {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: &Vec2) -> Vec2 {
        self.rem(*rhs)
    }
}

impl Rem<&Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: &Vec2) -> Vec2 {
        (*self).rem(*rhs)
    }
}

impl Rem<Vec2> for &Scalar {
    type Output = Vec2;
    #[inline]
    fn rem(self, rhs: Vec2) -> Vec2 {
        (*self).rem(rhs)
    }
}

impl AsRef<[Scalar; 2]> for Vec2 {
    #[inline]
    fn as_ref(&self) -> &[Scalar; 2] {
        unsafe { &*(self as *const Self as *const [Scalar; 2]) }
    }
}

impl AsMut<[Scalar; 2]> for Vec2 {
    #[inline]
    fn as_mut(&mut self) -> &mut [Scalar; 2] {
        unsafe { &mut *(self as *mut Self as *mut [Scalar; 2]) }
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = f.precision() {
            write!(f, "[{:.*}, {:.*}]", p, self.x, p, self.y)
        } else {
            write!(f, "[{}, {}]", self.x, self.y)
        }
    }
}

impl fmt::Debug for Vec2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple(stringify!(Vec2))
            .field(&self.x)
            .field(&self.y)
            .finish()
    }
}

impl From<[Scalar; 2]> for Vec2 {
    #[inline]
    fn from(a: [Scalar; 2]) -> Self {
        Self::new(a[0], a[1])
    }
}

impl From<Vec2> for [Scalar; 2] {
    #[inline]
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

impl From<(Scalar, Scalar)> for Vec2 {
    #[inline]
    fn from(t: (Scalar, Scalar)) -> Self {
        Self::new(t.0, t.1)
    }
}

impl From<Vec2> for (Scalar, Scalar) {
    #[inline]
    fn from(v: Vec2) -> Self {
        (v.x, v.y)
    }
}

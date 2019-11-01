use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Sub};

/// Types that have a "zero" value.
///
/// This trait is intended for use in conjunction with `Add`, as an identity:
/// `x + T::zero() == x`.
pub trait Zero: Copy {
    /// The "zero" (usually, additive identity) for this type.
    const ZERO: Self;
}

/// Types that have a "one" value.
///
/// This trait is intended for use in conjunction with `Mul`, as an identity:
/// `x * T::one() == x`.
pub trait One: Copy {
    /// The "one" (usually, multiplicative identity) for this type.
    const ONE: Self;
}

macro_rules! zero_one_impl {
    ($($t:ty)*) => ($(
        impl Zero for $t {
            const ZERO: Self = 0;
        }
        impl One for $t {
            const ONE: Self = 1;
        }
    )*)
}
zero_one_impl! { u8 u16 u32 u64 usize i8 i16 i32 i64 isize }

macro_rules! zero_one_impl_float {
    ($($t:ty)*) => ($(
         impl Zero for $t {
            const ZERO: Self = 0.0;
        }
        impl One for $t {
            const ONE: Self = 1.0;
        }
    )*)
}

zero_one_impl_float! { f32 f64 }

// todo: simd
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vec3<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Vec3 { x, y, z }
    }
}

impl<T> Vec3<T>
where
    T: Mul<Output = T> + Add<Output = T> + Into<f64> + Copy,
{
    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z)
            .into()
            .sqrt()
    }
}

impl<T> Vec3<T>
where
    T: Mul<Output = T>
        + Add<Output = T>
        + Into<f64>
        + One
        + Div<f64, Output = T>
        + MulAssign<T>
        + Copy,
{
    pub fn normalize(&mut self) {
        let f = T::ONE / self.length();
        self.x *= f;
        self.y *= f;
        self.z *= f;
    }
}

impl<T> Vec3<T>
where
    T: Copy + Mul<Output = T> + Sub<Output = T>,
{
    pub fn cross(&self, rhs: &Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }
}

impl<T> Default for Vec3<T>
where
    T: Default,
{
    fn default() -> Self {
        Vec3 {
            x: T::default(),
            y: T::default(),
            z: T::default(),
        }
    }
}

impl<T> Add<Self> for &Vec3<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Vec3<T>;

    fn add(self, rhs: &Vec3<T>) -> Self::Output {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl<T> AddAssign<&Vec3<T>> for Vec3<T>
where
    T: AddAssign<T> + Copy,
{
    fn add_assign(&mut self, rhs: &Vec3<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl<T> Sub<Self> for &Vec3<T>
where
    T: Sub<Output = T> + Copy,
{
    type Output = Vec3<T>;

    fn sub(self, rhs: &Vec3<T>) -> Self::Output {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

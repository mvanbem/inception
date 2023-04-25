use core::fmt::{Debug, Display};
use core::hash::Hash;

use num_traits::Num;

use crate::sealed::Sealed;

/// Unsigned `bitint` types.
///
/// There is one type implementing `UBitint` for each bit width from 1 to 128
/// inclusive.
pub trait UBitint:
    Copy
    + Debug
    + Display
    + Hash
    + Eq
    + Ord
    + Num
    + CheckedAdd
    + CheckedDiv
    + CheckedMul
    + CheckedRem
    + CheckedSub
    + WrappingAdd
    + WrappingMul
    + WrappingSub
    + Sized
    + Sealed
{
    /// The primitive type that this type wraps.
    type Primitive: From<Self>;

    /// The bit width of this type.
    const BITS: usize;
    /// The bit mask for the bits that may be set in values of this type.
    const MASK: Self::Primitive;

    /// The smallest value of this type.
    const MIN: Self;
    /// The largest value of this type.
    const MAX: Self;

    /// The value `0` represented in this type.
    const ZERO: Self;
    /// The value `1` represented in this type.
    const ONE: Self;

    /// Creates an unsigned `bitint` value from a primitive value if it is in
    /// range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    fn new(value: Self::Primitive) -> Option<Self>;

    /// Creates an unsigned `bitint` value by masking off the upper bits of a
    /// primitive value.
    ///
    /// This conversion is lossless if the value is in range for this type, as
    /// determined by [`is_in_range`](Self::is_in_range).
    fn new_masked(value: Self::Primitive) -> Self;

    /// Creates an unsigned `bitint` value from a primitive value without
    /// checking whether it is in range for this type.
    ///
    /// This is a zero-cost conversion.
    ///
    /// # Safety
    ///
    /// The value must be in range for this type, as determined by
    /// [`is_in_range`](Self::is_in_range).
    unsafe fn new_unchecked(value: Self::Primitive) -> Self;

    /// Converts the value to a primitive type.
    ///
    /// This is a zero-cost conversion. The result is in range for this type, as
    /// determined by [`is_in_range`](Self::is_in_range).
    fn to_primitive(self) -> Self::Primitive;

    /// Checks whether a primitive value is in range for this type.
    ///
    /// There are a few equivalent ways to express this check.
    ///
    /// - The unused most significant bits are clear: `(value & !Self::MASK) ==
    ///   0`
    /// - The value is between [`MIN`](Self::MIN) and [`MAX`](Self::MAX),
    ///   inclusive: `value >= Self::MIN.as_primitive() && value <=
    ///   Self::MAX.as_primitive()`
    ///
    fn is_in_range(value: Self::Primitive) -> bool;
}

/// `bitint` types that are the same width as a primitive integer type.
pub trait PrimitiveSizedBitint: UBitint + From<Self::Primitive> {
    /// Creates a `bitint` value from a primitive value of the same width.
    ///
    /// This is a zero-cost conversion.
    fn from_primitive(value: Self::Primitive) -> Self;
}

/// Checked integer addition. A generalization of [`num_traits::CheckedAdd`].
pub trait CheckedAdd<Rhs = Self>: num_traits::CheckedAdd {
    /// Checked integer addition. Computes `self + rhs`, returning `None` if
    /// overflow occurred.
    fn checked_add(self, rhs: Rhs) -> Option<Self::Output>;
}

/// Checked integer division. A generalization of [`num_traits::CheckedDiv`].
pub trait CheckedDiv<Rhs = Self>: num_traits::CheckedDiv {
    /// Checked integer division. Computes `self / rhs`, returning `None` if
    /// overflow occurred.
    fn checked_div(self, rhs: Rhs) -> Option<Self::Output>;
}

/// Checked integer multiplication. A generalization of [`num_traits::CheckedMul`].
pub trait CheckedMul<Rhs = Self>: num_traits::CheckedMul {
    /// Checked integer multiplication. Computes `self * rhs`, returning `None`
    /// if overflow occurred.
    fn checked_mul(self, rhs: Rhs) -> Option<Self::Output>;
}

/// Checked integer remainder. A generalization of [`num_traits::CheckedRem`].
pub trait CheckedRem<Rhs = Self>: num_traits::CheckedRem {
    /// Checked integer remainder. Computes `self % rhs`, returning `None` if
    /// overflow occurred.
    fn checked_rem(self, rhs: Rhs) -> Option<Self::Output>;
}

/// Checked integer subtraction. A generalization of [`num_traits::CheckedSub`].
pub trait CheckedSub<Rhs = Self>: num_traits::CheckedSub {
    /// Checked integer subtraction. Computes `self - rhs`, returning `None` if
    /// overflow occurred.
    fn checked_sub(self, rhs: Rhs) -> Option<Self::Output>;
}

/// Wrapping (modular) integer addition. A generalization of [`num_traits::WrappingAdd`].
pub trait WrappingAdd<Rhs = Self>: num_traits::WrappingAdd {
    /// Wrapping (modular) integer addition. Computes `self + rhs`, wrapping
    /// around at the boundary of the type.
    fn wrapping_add(self, rhs: Rhs) -> Self::Output;
}

/// Wrapping (modular) integer multiplication. A generalization of [`num_traits::WrappingMul`].
pub trait WrappingMul<Rhs = Self>: num_traits::WrappingMul {
    /// Wrapping (modular) integer multiplication. Computes `self * rhs`,
    /// wrapping around at the boundary of the type.
    fn wrapping_mul(self, rhs: Rhs) -> Self::Output;
}

/// Wrapping (modular) integer subtraction. A generalization of [`num_traits::WrappingSub`].
pub trait WrappingSub<Rhs = Self>: num_traits::WrappingSub {
    /// Wrapping (modular) integer subtraction. Computes `self - rhs`, wrapping
    /// around at the boundary of the type.
    fn wrapping_sub(self, rhs: Rhs) -> Self::Output;
}

#[cfg(feature = "unchecked_math")]
#[cfg_attr(feature = "_nightly", doc(cfg(unchecked_math)))]
/// Unchecked integer addition.
pub trait UncheckedAdd<Rhs = Self> {
    /// The resulting type.
    type Output;

    /// Unchecked integer addition. Computes `self + rhs`, assuming overflow
    /// cannot occur.
    ///
    /// # Safety
    ///
    /// The result must be in range for this type. For unsigned `bitint`s, this
    /// is as determined by [`UBitint::is_in_range`].
    unsafe fn unchecked_add(self, rhs: Rhs) -> Self::Output;
}

#[cfg(feature = "unchecked_math")]
#[cfg_attr(feature = "_nightly", doc(cfg(unchecked_math)))]
/// Unchecked integer multiplication.
pub trait UncheckedMul<Rhs = Self> {
    /// The resulting type.
    type Output;

    /// Unchecked integer multiplication. Computes `self * rhs`, assuming
    /// overflow cannot occur.
    ///
    /// # Safety
    ///
    /// The result must be in range for this type. For unsigned `bitint`s, this
    /// is as determined by [`UBitint::is_in_range`].
    unsafe fn unchecked_mul(self, rhs: Rhs) -> Self::Output;
}

#[cfg(feature = "unchecked_math")]
#[cfg_attr(feature = "_nightly", doc(cfg(unchecked_math)))]
/// Unchecked integer subtraction.
pub trait UncheckedSub<Rhs = Self> {
    /// The resulting type.
    type Output;

    /// Unchecked integer subtraction. Computes `self - rhs`, assuming overflow
    /// cannot occur.
    ///
    /// # Safety
    ///
    /// The result must be in range for this type. For unsigned `bitint`s, this
    /// is as determined by [`UBitint::is_in_range`].
    unsafe fn unchecked_sub(self, rhs: Rhs) -> Self::Output;
}

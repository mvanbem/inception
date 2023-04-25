//! The unsigned `bitint` types [`U1`] through [`U128`].

use core::fmt::{self, Display, Formatter};
use core::ops::{Add, Div, Mul, Rem, Sub};
use core::str::FromStr;

use num_traits::{Num, One, Zero};
use paste::paste;
use seq_macro::seq;

use crate::{
    CheckedAdd, CheckedDiv, CheckedMul, CheckedRem, CheckedSub, ParseBitintError,
    PrimitiveSizedBitint, RangeError, UBitint, WrappingAdd, WrappingMul, WrappingSub,
};
#[cfg(feature = "unchecked_math")]
use crate::{UncheckedAdd, UncheckedMul, UncheckedSub};

macro_rules! define_ubitint_type {
    ($a:literal..$b:literal: $primitive:ident; $flag:tt) => {
        seq!(N in $a..$b { define_ubitint_type!(N: $primitive; $flag); });
    };
    ($bits:literal: $primitive:ident; $flag:tt) => {
        paste! {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[doc = define_ubitint_type!(@type_doc $bits $primitive $flag)]
            #[repr(transparent)]
            pub struct [<U $bits>]($primitive);

            impl [<U $bits>] {
                /// Creates a `bitint` from a primitive value if it is in range
                /// for this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`UBitint::new`].
                #[inline(always)]
                #[must_use]
                pub const fn new(value: $primitive) -> Option<Self> {
                    if Self::is_in_range(value) {
                        Some(Self(value))
                    } else {
                        None
                    }
                }

                /// Creates a `bitint` by masking off the upper bits of a
                /// primitive value.
                ///
                /// This conversion is lossless if the value is in range for
                /// this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`UBitint::new_masked`].
                #[inline(always)]
                #[must_use]
                pub const fn new_masked(value: $primitive) -> Self {
                    Self(value & Self::MASK )
                }

                /// Creates a `bitint` from a primitive value without checking
                /// whether it is in range for this type.
                ///
                /// # Safety
                ///
                /// The value must be in range for this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of
                /// [`UBitint::new_unchecked`].
                #[inline(always)]
                #[must_use]
                pub const unsafe fn new_unchecked(value: $primitive) -> Self {
                    Self(value)
                }

                /// Converts the value to a primitive type.
                ///
                /// The result is in range for this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of
                /// [`UBitint::to_primitive`].
                #[inline(always)]
                #[must_use]
                pub const fn to_primitive(self) -> $primitive {
                    self.0
                }

                /// Checks whether a primitive value is in range for this type.
                ///
                /// There are a few equivalent ways to express this check.
                ///
                /// - The unused most significant bits are clear: `(value &
                ///   !Self::MASK) == 0`
                /// - The value is between [`MIN`](Self::MIN) and
                ///   [`MAX`](Self::MAX), inclusive: `value >=
                ///   Self::MIN.as_primitive() && value <=
                ///   Self::MAX.as_primitive()`
                ///
                /// This method is a `const` variant of
                /// [`UBitint::is_in_range`].
                pub const fn is_in_range(value: $primitive) -> bool {
                    value & !Self::MASK == 0
                }
            }

            impl crate::sealed::Sealed for [<U $bits>] {}

            impl UBitint for [<U $bits>] {
                type Primitive = $primitive;

                const BITS: usize = $bits;
                const MASK: $primitive = if $bits < $primitive::BITS {
                    (1 << $bits) - 1
                } else {
                    $primitive::MAX
                };

                const MIN: Self = Self::new_masked(0);
                const MAX: Self = Self::new_masked($primitive::MAX);

                const ZERO: Self = Self::new_masked(0);
                const ONE: Self = Self::new_masked(1);

                fn new(value: $primitive) -> Option<Self> {
                    Self::new(value)
                }

                fn new_masked(value: $primitive) -> Self {
                    Self::new_masked(value)
                }

                unsafe fn new_unchecked(value: $primitive) -> Self {
                    Self::new_unchecked(value)
                }

                fn to_primitive(self) -> $primitive {
                    self.to_primitive()
                }

                fn is_in_range(value: $primitive) -> bool {
                    Self::is_in_range(value)
                }
            }

            impl Zero for [<U $bits>] {
                fn zero() -> Self {
                    Self::ZERO
                }

                fn is_zero(&self) -> bool {
                    *self == Self::ZERO
                }
            }

            impl One for [<U $bits>] {
                fn one() -> Self {
                    Self::ONE
                }

                fn is_one(&self) -> bool {
                    *self == Self::ONE
                }
            }

            impl FromStr for [<U $bits>] {
                type Err = ParseBitintError;

                fn from_str(s: &str) -> Result<Self, ParseBitintError> {
                    Self::new($primitive::from_str(s)?)
                        .ok_or_else(|| RangeError(()).into())
                }
            }

            impl Num for [<U $bits>] {
                type FromStrRadixErr = ParseBitintError;

                fn from_str_radix(
                    str: &str,
                    radix: u32
                ) -> Result<Self, ParseBitintError> {
                    Self::new($primitive::from_str_radix(str, radix)?)
                        .ok_or_else(|| RangeError(()).into())
                }
            }

            impl Display for [<U $bits>] {
                fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                    write!(f, "{}", self.to_primitive())
                }
            }

            impl From<[<U $bits>]> for $primitive {
                #[inline(always)]
                fn from(value: [<U $bits>]) -> Self {
                    value.to_primitive()
                }
            }

            define_ubitint_type!(@impl_from [<U $bits>] $primitive $flag);
            define_ubitint_type!(@impl_op [<U $bits>] $primitive Add::add + ext);
            define_ubitint_type!(@impl_op [<U $bits>] $primitive Div::div /);
            define_ubitint_type!(@impl_op [<U $bits>] $primitive Mul::mul * ext);
            define_ubitint_type!(@impl_op [<U $bits>] $primitive Rem::rem %);
            define_ubitint_type!(@impl_op [<U $bits>] $primitive Sub::sub - ext);
        }
    };
    (@type_doc $bits:literal $primitive:ident upper_bits_clear) => {
        concat!(
            "The ", stringify!($bits), "-bit unsigned `bitint` type.",
            "\n\n",
            "# Layout",
            "\n\n",
            "This type is `#[repr(transparent)]` to [`", stringify!($primitive), "`], but imposes ",
            "additional invariants.",
            "\n\n",
            "# Invariants",
            "\n\n",
            "The value is represented in the least significant bits of a [`",
            stringify!($primitive),
            "`]. The unused most significant bits are always clear.",
        )
    };
    (@type_doc $bits:literal $primitive:ident any_bit_pattern) => {
        concat!(
            "The ", stringify!($bits), "-bit unsigned `bitint` type.",
            "\n\n",
            "# Layout",
            "\n\n",
            "This type is `#[repr(transparent)]` to [`", stringify!($primitive), "`].",
        )
    };
    (@impl_from $self:ident $primitive:ident any_bit_pattern) => {
        paste! {
            impl PrimitiveSizedBitint for $self {
                fn from_primitive(value: $primitive) -> Self {
                    Self(value)
                }
            }

            impl From<$primitive> for $self {
                #[inline(always)]
                fn from(value: $primitive) -> Self {
                    Self::from_primitive(value)
                }
            }
        }
    };
    (@impl_from $self:ident $primitive:ident upper_bits_clear) => {
        paste! {
            impl TryFrom<$primitive> for $self {
                type Error = RangeError;

                #[inline(always)]
                fn try_from(value: $primitive) -> Result<Self, RangeError> {
                    Self::new(value).ok_or(RangeError(()))
                }
            }
        }
    };
    (@impl_op $self:ident $primitive:ident $trait:ident::$method:ident $op:tt $($ext:tt)?) => {
        paste! {
            impl $trait<$primitive> for $self {
                type Output = Self;

                fn $method(self, rhs: $primitive) -> Self {
                    let result = self.to_primitive() $op rhs;
                    // Perform a sentinel operation with overflow behavior that
                    // depends on the build configuration:
                    // - With overflow-checks enabled, this panics on overflow.
                    //   Even though the result is unused, optimizer should
                    //   retain its side effects.
                    // - With overflow-checks disabled, this wraps on overflow.
                    //   The result is unused and there are no side effects, so
                    //   it is likely to be optimized out entirely.
                    let _ = result + ($primitive::MAX - Self::MAX.0);
                    // We must either wrap or panic here. The unchecked
                    // constructor is unsafe so calling it would be unsound.
                    Self::new_masked(result)
                }
            }

            impl $trait for $self {
                type Output = Self;

                fn $method(self, rhs: Self) -> Self {
                    self $op rhs.to_primitive()
                }
            }

            define_ubitint_type!(@impl_checked_op $self $primitive $trait::$method);
            define_ubitint_type!(@impl_wrapping_op $self $primitive $trait::$method $($ext)?);
            define_ubitint_type!(@impl_unchecked_op $self $primitive $trait::$method $($ext)?);
        }
    };
    (@impl_checked_op $self:ident $primitive:ident $trait:ident::$method:ident) => {
        paste! {
            impl [<Checked $trait>]<$primitive> for $self {
                fn [<checked_ $method>](self, rhs: $primitive) -> Option<Self> {
                    self.to_primitive().[<checked_ $method>](rhs).and_then(Self::new)
                }
            }

            impl [<Checked $trait>] for $self {
                fn [<checked_ $method>](self, rhs: Self) -> Option<Self> {
                    self.[<checked_ $method>](rhs.to_primitive())
                }
            }

            impl num_traits::[<Checked $trait>] for $self {
                fn [<checked_ $method>](&self, v: &Self) -> Option<Self> {
                    [<Checked $trait>]::[<checked_ $method>](*self, *v)
                }
            }
        }
    };
    (@impl_wrapping_op $self:ident $primitive:ident $trait:ident::$method:ident) => {};
    (@impl_wrapping_op $self:ident $primitive:ident $trait:ident::$method:ident ext) => {
        paste! {
            impl [<Wrapping $trait>]<$primitive> for $self {
                fn [<wrapping_ $method>](self, rhs: $primitive) -> Self {
                    Self::new_masked(self.to_primitive().[<wrapping_ $method>](rhs))
                }
            }

            impl [<Wrapping $trait>] for $self {
                fn [<wrapping_ $method>](self, rhs: Self) -> Self {
                    self.[<wrapping_ $method>](rhs.to_primitive())
                }
            }

            impl num_traits::[<Wrapping $trait>] for $self {
                fn [<wrapping_ $method>](&self, v: &Self) -> Self {
                    [<Wrapping $trait>]::[<wrapping_ $method>](*self, *v)
                }
            }
        }
    };
    (@impl_unchecked_op $self:ident $primitive:ident $trait:ident::$method:ident) => {};
    (@impl_unchecked_op $self:ident $primitive:ident $trait:ident::$method:ident ext) => {
        paste! {
            #[cfg(feature = "unchecked_math")]
            #[cfg_attr(feature = "_nightly", doc(cfg(unchecked_math)))]
            impl [<Unchecked $trait>]<$primitive> for $self {
                type Output = Self;

                unsafe fn [<unchecked_ $method>](self, rhs: $primitive) -> Self {
                    Self::new_unchecked(self.to_primitive().[<unchecked_ $method>](rhs))
                }
            }

            #[cfg(feature = "unchecked_math")]
            #[cfg_attr(feature = "_nightly", doc(cfg(unchecked_math)))]
            impl [<Unchecked $trait>] for $self {
                type Output = Self;

                unsafe fn [<unchecked_ $method>](self, rhs: Self) -> Self {
                    self.[<unchecked_ $method>](rhs.to_primitive())
                }
            }
        }
    };
}

define_ubitint_type!(1..8: u8; upper_bits_clear);
define_ubitint_type!(8: u8; any_bit_pattern);
define_ubitint_type!(9..16: u16; upper_bits_clear);
define_ubitint_type!(16: u16; any_bit_pattern);
define_ubitint_type!(17..32: u32; upper_bits_clear);
define_ubitint_type!(32: u32; any_bit_pattern);
define_ubitint_type!(33..64: u64; upper_bits_clear);
define_ubitint_type!(64: u64; any_bit_pattern);
define_ubitint_type!(65..128: u128; upper_bits_clear);
define_ubitint_type!(128: u128; any_bit_pattern);

impl From<bool> for U1 {
    fn from(value: bool) -> Self {
        // SAFETY: `bool` and `U1` have the same size (1), alignment (1), and
        // valid bit patterns (0u8 and 1u8).
        unsafe { core::mem::transmute(value) }
    }
}

impl From<U1> for bool {
    fn from(value: U1) -> Self {
        // SAFETY: `bool` and `U1` have the same size (1), alignment (1), and
        // valid bit patterns (0u8 and 1u8).
        unsafe { core::mem::transmute(value) }
    }
}

/// A type-level function returning a [`UBitint`].
pub trait FnUBitint {
    /// The resulting type.
    type Type: UBitint;
}

/// Maps each bit width to its corresponding [`UBitint`] type.
pub enum UBitintForWidth<const WIDTH: usize> {}

macro_rules! impl_ubitint_for_width {
    ($width:literal) => {
        paste! {
            impl FnUBitint for UBitintForWidth<$width> {
                type Type = [<U $width>];
            }
        }
    };
}
seq!(N in 1..=128 { impl_ubitint_for_width!(N); });

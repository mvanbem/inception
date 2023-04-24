//! The unsigned `bitint` types [`U1`] through [`U128`].

use paste::paste;
use seq_macro::seq;

use crate::{PrimitiveSizedBitint, RangeError, Result, UBitint};

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
                /// Creates a `bitint` from a primitive value if it is in range for this type, as
                /// determined by [`is_in_range`](Self::is_in_range).
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

                /// Creates a `bitint` by masking off the upper bits of a primitive value.
                ///
                /// This conversion is lossless if the value is in range for this type, as
                /// determined by [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`UBitint::new_masked`].
                #[inline(always)]
                #[must_use]
                pub const fn new_masked(value: $primitive) -> Self {
                    Self(value & Self::MASK )
                }

                /// Creates a `bitint` from a primitive value without checking whether it is in
                /// range for this type.
                ///
                /// # Safety
                ///
                /// The value must be in range for this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`UBitint::new_unchecked`].
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
                /// This method is a `const` variant of [`UBitint::to_primitive`].
                #[inline(always)]
                #[must_use]
                pub const fn to_primitive(self) -> $primitive {
                    self.0
                }

                /// Checks whether a primitive value is in range for this type.
                ///
                /// There are a few equivalent ways to express this check.
                ///
                /// - The unused most significant bits are clear: `(value & !Self::MASK) == 0`
                /// - The value is between [`MIN`](Self::MIN) and [`MAX`](Self::MAX), inclusive:
                ///   `value >= Self::MIN.as_primitive() && value <= Self::MAX.as_primitive()`
                ///
                /// This method is a `const` variant of [`UBitint::is_in_range`].
                pub const fn is_in_range(value: $primitive) -> bool {
                    value & !Self::MASK == 0
                }

                define_ubitint_type!(@ops $primitive add "addition" +);
                define_ubitint_type!(@ops $primitive sub "subtraction" -);
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

                fn min() -> Self { Self::MIN }

                fn max() -> Self { Self::MAX }

                fn zero() -> Self { Self::ZERO }

                fn one() -> Self { Self::ONE }
            }

            impl From<[<U $bits>]> for $primitive {
                #[inline(always)]
                fn from(value: [<U $bits>]) -> Self {
                    value.to_primitive()
                }
            }

            define_ubitint_type!(@flag_impls $bits $primitive $flag);
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
    (@ops $primitive:ident $name:ident $desc:literal $op:tt) => {
        paste! {
            #[doc = concat!(
                "Checked integer ", $desc, ". Computes `self ", stringify!($op), " rhs`, ",
                "returning `None` if overflow occurred.",
            )]
            #[inline(always)]
            #[must_use]
            pub fn [<checked_ $name>]<Rhs: Into<$primitive>>(self, rhs: Rhs) -> Option<Self> {
                match self.0.[<checked_ $name>](rhs.into()) {
                    Some(value) => Self::new(value),
                    None => None,
                }
            }

            #[doc = concat!(
                "Wrapping (modular) ", $desc, ". Computes `self ", stringify!($op), " rhs`, ",
                "wrapping around at the boundary of the type.",
            )]
            #[inline(always)]
            #[must_use]
            pub fn [<wrapping_ $name>]<Rhs: Into<$primitive>>(self, rhs: Rhs) -> Self {
                Self::new_masked(self.0.[<wrapping_ $name>](rhs.into()))
            }

            #[doc = concat!(
                "Unchecked integer ", $desc, ". Computes `self ", stringify!($op), " rhs`, ",
                "assuming overflow cannot occur.",
                "\n\n",
                "# Safety",
                "\n\n",
                "The intermediate [`", stringify!($primitive), "`] operation must not overflow. The ",
                "result must be in range for this type, as determined by ",
                "[`is_in_range`](Self::is_in_range)",
            )]
            #[inline(always)]
            #[must_use]
            pub unsafe fn [<unchecked_ $name>]<Rhs: Into<$primitive>>(self, rhs: Rhs) -> Self {
                Self::new_unchecked(self.0 $op rhs.into())
            }
        }
    };
    (@flag_impls $bits:literal $primitive:ident any_bit_pattern) => {
        paste! {
            impl PrimitiveSizedBitint for [<U $bits>] {
                fn from_primitive(value: $primitive) -> Self {
                    Self(value)
                }
            }

            impl From<$primitive> for [<U $bits>] {
                #[inline(always)]
                fn from(value: $primitive) -> Self {
                    Self::from_primitive(value)
                }
            }
        }
    };
    (@flag_impls $bits:literal $primitive:ident upper_bits_clear) => {
        paste! {
            impl TryFrom<$primitive> for [<U $bits>] {
                type Error = RangeError;

                #[inline(always)]
                fn try_from(value: $primitive) -> Result<Self> {
                    Self::new(value).ok_or(RangeError(()))
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
        // SAFETY: `bool` and `U1` have the same size (1), alignment (1), and valid bit patterns
        // (0u8 and 1u8).
        unsafe { core::mem::transmute(value) }
    }
}

impl From<U1> for bool {
    fn from(value: U1) -> Self {
        // SAFETY: `bool` and `U1` have the same size (1), alignment (1), and valid bit patterns
        // (0u8 and 1u8).
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

//! The unsigned narrow integer types: [`U1`] through [`U127`].

use paste::paste;
use seq_macro::seq;

use crate::{NarrowInteger, RangeError, Result};

macro_rules! define_narrow_unsigned_integer {
    ($bits:literal: $repr:ident) => {
        paste! {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[doc = concat!(
                "The ", $bits, "-bit unsigned integer type.",
                "\n\n",
                "# Invariants",
                "\n\n",
                "The value is represented in the least significant bits of a [`", stringify!($repr),
                "`]. The unused most significant bits are always clear.",
            )]
            #[repr(transparent)]
            pub struct [<U $bits>]($repr);

            impl [<U $bits>] {
                /// Creates a narrow integer from a primitive value if it is in range for this type,
                /// as determined by [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`NarrowInteger::new`].
                #[inline(always)]
                #[must_use]
                pub const fn new(value: $repr) -> Option<Self> {
                    if Self::is_in_range(value) {
                        Some(Self(value))
                    } else {
                        None
                    }
                }

                /// Creates a narrow integer by masking off the upper bits of a primitive value.
                ///
                /// This conversion is lossless if the value is in range for this type, as
                /// determined by [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`NarrowInteger::new_masked`].
                #[inline(always)]
                #[must_use]
                pub const fn new_masked(value: $repr) -> Self {
                    Self(value & Self::MASK )
                }

                /// Creates a narrow integer from a primitive value without checking whether it is
                /// in range for this type.
                ///
                /// # Safety
                ///
                /// The value must be in range for this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`NarrowInteger::new_unchecked`].
                #[inline(always)]
                #[must_use]
                pub const unsafe fn new_unchecked(value: $repr) -> Self {
                    Self(value)
                }

                /// Converts the value to a primitive type.
                ///
                /// The result is in range for this type, as determined by
                /// [`is_in_range`](Self::is_in_range).
                ///
                /// This method is a `const` variant of [`NarrowInteger::as_primitive`].
                #[inline(always)]
                #[must_use]
                pub const fn [<as_ $repr>](self) -> $repr {
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
                /// This method is a `const` variant of [`NarrowInteger::is_in_range`].
                pub const fn is_in_range(value: $repr) -> bool {
                    value & !Self::MASK == 0
                }

                define_narrow_unsigned_integer!(@ops $repr add "addition" +);
                define_narrow_unsigned_integer!(@ops $repr sub "subtraction" -);
            }

            impl NarrowInteger for [<U $bits>] {
                type Primitive = $repr;

                const BITS: usize = $bits;
                const MASK: $repr = (1 << $bits) - 1;

                const MIN: Self = Self::new_masked(0);
                const MAX: Self = Self::new_masked($repr::MAX);

                const ZERO: Self = Self::new_masked(0);
                const ONE: Self = Self::new_masked(1);

                fn new(value: $repr) -> Option<Self> {
                    Self::new(value)
                }

                fn new_masked(value: $repr) -> Self {
                    Self::new_masked(value)
                }

                unsafe fn new_unchecked(value: $repr) -> Self {
                    Self::new_unchecked(value)
                }

                fn as_primitive(self) -> $repr {
                    self.[<as_ $repr>]()
                }

                fn is_in_range(value: $repr) -> bool {
                    Self::is_in_range(value)
                }

                fn min() -> Self { Self::MIN }

                fn max() -> Self { Self::MAX }

                fn zero() -> Self { Self::ZERO }

                fn one() -> Self { Self::ONE }
            }

            impl TryFrom<$repr> for [<U $bits>] {
                type Error = RangeError;

                #[inline(always)]
                #[must_use]
                fn try_from(value: $repr) -> Result<Self> {
                    Self::new(value).ok_or(RangeError(()))
                }
            }

            impl From<[<U $bits>]> for $repr {
                #[inline(always)]
                #[must_use]
                fn from(value: [<U $bits>]) -> Self {
                    value.[<as_ $repr>]()
                }
            }
        }
    };
    (@ops $repr:ident $name:ident $desc:literal $op:tt) => {
        paste! {
            #[doc = concat!(
                "Checked integer ", $desc, ". Computes `self ", stringify!($op), " rhs`, ",
                "returning `None` if overflow occurred.",
            )]
            #[inline(always)]
            #[must_use]
            pub fn [<checked_ $name>]<Rhs: Into<$repr>>(self, rhs: Rhs) -> Option<Self> {
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
            pub fn [<wrapping_ $name>]<Rhs: Into<$repr>>(self, rhs: Rhs) -> Self {
                Self::new_masked(self.0.[<wrapping_ $name>](rhs.into()))
            }

            #[doc = concat!(
                "Unchecked integer ", $desc, ". Computes `self ", stringify!($op), " rhs`, ",
                "assuming overflow cannot occur.",
                "\n\n",
                "# Safety",
                "\n\n",
                "The intermediate [`", stringify!($repr), "`] operation must not overflow. The ",
                "result must be in range for this type, as determined by ",
                "[`is_in_range`](Self::is_in_range)",
            )]
            #[inline(always)]
            #[must_use]
            pub unsafe fn [<unchecked_ $name>]<Rhs: Into<$repr>>(self, rhs: Rhs) -> Self {
                Self::new_unchecked(self.0 $op rhs.into())
            }
        }
    };
}

seq!(N in 1..8 { define_narrow_unsigned_integer!(N: u8); });
seq!(N in 9..16 { define_narrow_unsigned_integer!(N: u16); });
seq!(N in 17..32 { define_narrow_unsigned_integer!(N: u32); });
seq!(N in 33..64 { define_narrow_unsigned_integer!(N: u64); });
seq!(N in 64..128 { define_narrow_unsigned_integer!(N: u128); });

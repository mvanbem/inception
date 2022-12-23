use paste::paste;
use seq_macro::seq;

use crate::InvalidBitPattern;

pub trait NarrowInteger {
    /// The underlying primitive integer type.
    type T;

    /// The bit width of this type.
    const BITS: usize;
    /// The bit mask that precisely masks the bits that may be set in values of this type.
    const MASK: Self::T;
}

macro_rules! define_narrow_unsigned_integer {
    ($bits:literal: $underlying:ident) => {
        paste! {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[doc = concat!(
                "The ", $bits, "-bit unsigned integer type.",
                "\n\n",
                "It is represented in the least ",
                "significant bits of a `", stringify!($underlying), "`. The upper bits are always ",
                "clear, and the safety of unsafe code may rely on this invariant.",
            )]
            #[repr(transparent)]
            pub struct [<U $bits>]($underlying);

            impl [<U $bits>] {
                #[doc = concat!(
                    "Creates a `U", stringify!($bits), "` if the given value is in range.",
                )]
                #[inline(always)]
                pub const fn new(value: $underlying) -> Option<Self> {
                    if value & !Self::MASK == 0 {
                        Some(Self(value))
                    } else {
                        None
                    }
                }

                #[doc = concat!(
                    "Creates a `U", stringify!($bits), "` by masking off the upper bits.",
                )]
                #[inline(always)]
                pub const fn new_masked(value: $underlying) -> Self {
                    Self(value & Self::MASK )
                }

                #[doc = concat!(
                    "Creates a `U", stringify!($bits), "` without checking whether the value is ",
                    "in range. This may break assumptions in unsafe code if the value is out ",
                    "of range.",
                    "\n\n",
                    "# Safety",
                    "\n\n",
                    "The value must be less than [`Self::MASK`].",
                )]
                #[inline(always)]
                pub const unsafe fn new_unchecked(value: $underlying) -> Self {
                    Self(value)
                }

                #[doc = concat!("Returns the value as a primitive ", stringify!($underlying), ".")]
                #[inline(always)]
                pub const fn [<as_ $underlying>](self) -> $underlying {
                    self.0
                }
            }

            impl NarrowInteger for [<U $bits>] {
                type T = $underlying;

                const BITS: usize = $bits;
                const MASK: $underlying = (1 << $bits) - 1;
            }

            impl TryFrom<$underlying> for [<U $bits>] {
                type Error = InvalidBitPattern;

                #[inline(always)]
                fn try_from(value: $underlying) -> Result<Self, InvalidBitPattern> {
                    Self::new(value).ok_or(InvalidBitPattern)
                }
            }

            impl From<[<U $bits>]> for $underlying {
                #[inline(always)]
                fn from(value: [<U $bits>]) -> Self {
                    value.[<as_ $underlying>]()
                }
            }
        }
    };
}

seq!(N in 1..8 { define_narrow_unsigned_integer!(N: u8); });
seq!(N in 9..16 { define_narrow_unsigned_integer!(N: u16); });
seq!(N in 17..32 { define_narrow_unsigned_integer!(N: u32); });
seq!(N in 33..64 { define_narrow_unsigned_integer!(N: u64); });
seq!(N in 64..128 { define_narrow_unsigned_integer!(N: u128); });

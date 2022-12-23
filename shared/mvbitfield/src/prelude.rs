use paste::paste;
use seq_macro::seq;

#[doc(no_inline)]
pub use crate::mvbitfield;

seq!(N in 1..8 { paste! { pub use crate::narrow_integer::[<U N>]; } });
seq!(N in 9..16 { paste! { pub use crate::narrow_integer::[<U N>]; } });
seq!(N in 17..32 { paste! { pub use crate::narrow_integer::[<U N>]; } });
seq!(N in 33..64 { paste! { pub use crate::narrow_integer::[<U N>]; } });
seq!(N in 65..128 { paste! { pub use crate::narrow_integer::[<U N>]; } });

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
#![feature(doc_cfg)]
#![no_std]

pub use narrow_integer;

#[cfg(doc)]
pub mod doc;

pub mod prelude;

#[doc(hidden)]
pub mod __private {
    pub use mvbitfield_macros::bitfield;
}

/// Generates a bitfield struct.
///
/// See the [`doc`] module for an [overview of concepts and terms](doc::overview) and
/// [examples](doc::example).
///
#[doc = include_str!("../syntax.md")]
#[macro_export]
macro_rules! bitfield {
    ($($tt:tt)*) => {
        $crate::__private::bitfield! { ($crate, $($tt)*) }
    };
}

#[test]
fn trybuild_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests_error/*.rs");
}

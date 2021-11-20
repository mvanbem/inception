#[repr(C)]
pub struct AlignedAs<Align, Bytes: ?Sized> {
    pub _align: [Align; 0],
    pub bytes: Bytes,
}

macro_rules! include_bytes_align_as {
    ($align_ty:ty, $path:literal) => {{
        use $crate::include_bytes_align::AlignedAs;

        static ALIGNED: &AlignedAs<$align_ty, [u8]> = &AlignedAs {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}

macro_rules! include_bytes_align {
    ($align_bytes:expr, $path:literal) => {{
        use $crate::include_bytes_align::AlignedAs;

        #[repr(align($align_bytes))]
        struct Aligned;

        static ALIGNED: &AlignedAs<Aligned, [u8]> = &AlignedAs {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}
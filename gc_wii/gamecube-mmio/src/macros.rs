macro_rules! mmio_device {
    // Top-level matcher.
    (
        doc_name: $doc_name:literal,
        struct_name: $struct_name:ident,
        base: $base:literal,
        size: $size:literal,
        regs: {
            $(
                $reg_name:ident:
                $reg_type:tt
                $(= $reg_access:tt $(($indexed:tt))?)?
            ),*
            $(,)?
        },
    ) => {
        #[repr(C)]
        struct RegisterBlock {
            $($reg_name: $reg_type,)*
        }

        const _: () = assert!(::core::mem::size_of::<RegisterBlock>() == $size);

        #[derive(Clone, Copy)]
        #[doc = concat!(
            "Represents permission to access the ",
            stringify!($struct_name),
            " MMIO device.",
        )]
        pub struct $struct_name<'reg> {
            _phantom_lifetime: ::core::marker::PhantomData<&'reg ()>,
        }

        impl<'reg> $struct_name<'reg> {
            const PTR: *mut RegisterBlock = $base as _;

            pub fn new(root: crate::permission::PermissionRoot) -> Self {
                let _ = root;
                Self {
                    _phantom_lifetime: ::core::marker::PhantomData,
                }
            }

            $(
                mmio_device! { @reg_accessors $reg_name ($reg_type) $($reg_access $($indexed)?)? }
            )*
        }
    };

    // Dispatch on access specifiers.
    (@reg_accessors $name:ident ($type:ty)) => {};
    (@reg_accessors $name:ident ($type:ty) ro) => {
        mmio_device! { @read $name $type }
    };
    (@reg_accessors $name:ident ($type:tt) ro indexed) => {
        mmio_device! { @read_indexed $name $type }
    };
    (@reg_accessors $name:ident ($type:ty) wo) => {
        mmio_device! { @write $name $type }
    };
    (@reg_accessors $name:ident ($type:tt) wo indexed) => {
        mmio_device! { @write_indexed $name $type }
    };
    (@reg_accessors $name:ident ($type:ty) rw) => {
        mmio_device! { @read $name $type }
        mmio_device! { @write $name $type }
        mmio_device! { @modify $name $type }
    };
    (@reg_accessors $name:ident ($type:tt) rw indexed) => {
        mmio_device! { @read_indexed $name $type }
        mmio_device! { @write_indexed $name $type }
        mmio_device! { @modify_indexed $name $type }
    };

    // Non-indexed read implementation.
    (@read $name:ident $type:ty) => {
        ::paste::paste! {
            pub fn [<read_ $name>](&self) -> $type {
                unsafe {
                    ::core::ptr::read_volatile(
                        ::memoffset::raw_field!(Self::PTR, RegisterBlock, $name),
                    )
                }
            }
        }
    };

    // Non-indexed write implementation.
    (@write $name:ident $type:ty) => {
        ::paste::paste! {
            pub fn [<write_ $name>](&self, value: $type) {
                unsafe {
                    ::core::ptr::write_volatile(
                        ::memoffset::raw_field!(Self::PTR, RegisterBlock, $name).cast_mut(),
                        value,
                    );
                }
            }
        }
    };

    // Non-indexed modify implementation.
    (@modify $name:ident $type:ty) => {
        ::paste::paste! {
            pub fn [<modify_ $name>](
                &self,
                u: crate::uninterruptible::Uninterruptible,
                f: impl FnOnce($type) -> $type,
            ) {
                let _ = u;
                self.[<write_ $name>](f(self.[<read_ $name>]()));
            }
        }
    };

    // Indexed read implementation.
    (@read_indexed $name:ident [$type:ty; $count:literal]) => {
        ::paste::paste! {
            pub fn [<read_ $name>](&self, index: mmio_device!(@log2 $count)) -> $type {
                unsafe {
                    ::core::ptr::read_volatile(
                        ::memoffset::raw_field!(Self::PTR, RegisterBlock, $name)
                            .cast::<$type>()
                            .offset(<mmio_device!(@log2 $count)>::as_u8(index) as isize),
                    )
                }
            }
        }

        ::seq_macro::seq!(N in 0..$count {
            ::paste::paste! {
                pub fn [<read_ $name _ N>](&self) -> $type {
                    self.[<read_ $name>](<mmio_device!(@log2 $count)>::new_masked(N))
                }
            }
        });
    };

    // Indexed write implementation.
    (@write_indexed $name:ident [$type:ty; $count:literal]) => {
        ::paste::paste! {
            pub fn [<write_ $name>](&self, index: mmio_device!(@log2 $count), value: $type) {
                unsafe {
                    ::core::ptr::write_volatile(
                        ::memoffset::raw_field!(Self::PTR, RegisterBlock, $name)
                            .cast_mut()
                            .cast::<$type>()
                            .offset(<mmio_device!(@log2 $count)>::as_u8(index) as isize),
                        value,
                    );
                }
            }
        }

        ::seq_macro::seq!(N in 0..$count {
            ::paste::paste! {
                pub fn [<write_ $name _ N>](&self, value: $type) {
                    self.[<write_ $name>](<mmio_device!(@log2 $count)>::new_masked(N), value);
                }
            }
        });
    };

    // Indexed modify implementation.
    (@modify_indexed $name:ident [$type:ty; $count:literal]) => {
        ::paste::paste! {
            pub fn [<modify_ $name>](
                &self,
                u: crate::uninterruptible::Uninterruptible,
                index: mmio_device!(@log2 $count),
                f: impl FnOnce($type) -> $type,
            ) {
                let _ = u;
                self.[<write_ $name>](index, f(self.[<read_ $name>](index)));
            }
        }

        ::seq_macro::seq!(N in 0..$count {
            ::paste::paste! {
                pub fn [<modify_ $name _ N>](
                    &self,
                    u: crate::uninterruptible::Uninterruptible,
                    f: impl FnOnce($type) -> $type,
                ) {
                    let _ = u;
                    self.[<write_ $name _ N>](f(self.[<read_ $name _ N>]()));
                }
            }
        });
    };

    // Map array length to narrow integer types for indexing.
    (@log2 2) => { ::mvbitfield::narrow_integer::U1 };
    (@log2 4) => { ::mvbitfield::narrow_integer::U2 };
    (@log2 8) => { ::mvbitfield::narrow_integer::U3 };
    (@log2 16) => { ::mvbitfield::narrow_integer::U4 };
    (@log2 32) => { ::mvbitfield::narrow_integer::U5 };
    (@log2 64) => { ::mvbitfield::narrow_integer::U6 };
    (@log2 128) => { ::mvbitfield::narrow_integer::U7 };
    (@log2 256) => { ::mvbitfield::narrow_integer::U8 };
}

use core::cell::UnsafeCell;

use aligned::{Aligned, A32};

pub struct Framebuffer {
    data: Aligned<A32, UnsafeCell<[u8; Self::SIZE]>>,
}

impl Framebuffer {
    pub const SIZE: usize = 640 * 480 * 2;

    pub const fn zero() -> Self {
        Self {
            data: Aligned(UnsafeCell::new([0; Self::SIZE])),
        }
    }

    pub fn as_ptr(&self) -> *mut () {
        self.data.get().cast()
    }
}

unsafe impl Sync for Framebuffer {}

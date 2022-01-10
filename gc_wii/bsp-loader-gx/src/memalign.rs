#![allow(dead_code)]

use core::slice::{from_raw_parts, from_raw_parts_mut};

use libc::c_void;
use ogc_sys::DCFlushRange;

pub struct Memalign<const ALIGN: usize> {
    ptr: *mut c_void,
    size: usize,
}

impl<const ALIGN: usize> Memalign<ALIGN> {
    pub fn new(size: usize) -> Self {
        let ptr = unsafe { libc::memalign(ALIGN, size) };
        assert!(!ptr.is_null());
        Self { ptr, size }
    }

    pub fn as_void_ptr(&self) -> *const c_void {
        self.ptr as *const c_void
    }

    pub fn as_void_ptr_mut(&self) -> *mut c_void {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn as_ref(&self) -> &[u8] {
        unsafe { from_raw_parts(self.ptr as *const u8, self.size) }
    }

    pub fn as_mut(&mut self) -> &mut [u8] {
        unsafe { from_raw_parts_mut(self.ptr as *mut u8, self.size) }
    }

    pub unsafe fn dc_flush(&self) {
        unsafe {
            DCFlushRange(self.ptr, self.size as u32);
        }
    }
}

impl<const ALIGN: usize> Clone for Memalign<ALIGN> {
    fn clone(&self) -> Self {
        let result = Self::new(self.size);
        unsafe { libc::memcpy(result.ptr, self.ptr, self.size) };
        result
    }
}

impl<const ALIGN: usize> Drop for Memalign<ALIGN> {
    fn drop(&mut self) {
        unsafe { libc::free(self.ptr) }
    }
}

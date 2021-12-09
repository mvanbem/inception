#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

use core::mem::{align_of, size_of};
use core::slice::from_raw_parts;

fn is_aligned<T>(ptr: *const T) -> bool {
    (ptr as usize) % align_of::<T>() == 0
}

/// Marker trait for fully occupied types. A fully occupied type is valid for any state of the bits
/// in its representation.
///
/// A fully occupied type may be safely extracted from any correctly aligned byte slice.
pub unsafe trait FullyOccupied {}

// SAFETY: Primitive integer types are fully occupied.
unsafe impl FullyOccupied for u8 {}
unsafe impl FullyOccupied for u16 {}
unsafe impl FullyOccupied for u32 {}
unsafe impl FullyOccupied for u64 {}
unsafe impl FullyOccupied for u128 {}
unsafe impl FullyOccupied for i8 {}
unsafe impl FullyOccupied for i16 {}
unsafe impl FullyOccupied for i32 {}
unsafe impl FullyOccupied for i64 {}
unsafe impl FullyOccupied for i128 {}

// SAFETY: Floating-point types are fully occupied.
unsafe impl FullyOccupied for f32 {}
unsafe impl FullyOccupied for f64 {}

// SAFETY: Arrays of fully-occupied types are fully occupied.
unsafe impl<T: FullyOccupied, const N: usize> FullyOccupied for [T; N] {}

pub fn as_bytes<T: FullyOccupied>(value: &T) -> &[u8] {
    unsafe { from_raw_parts(value as *const T as *const u8, size_of::<T>()) }
}

pub fn slice_as_bytes<T: FullyOccupied>(values: &[T]) -> &[u8] {
    unsafe { from_raw_parts(values.as_ptr() as *const u8, values.len() * size_of::<T>()) }
}

/// Reinteprets a prefix of a byte slice as a value of T.
///
/// # Panics
///
/// Panics if any of the following preconditions fail:
///
/// - `data.len()` must be at least `size_of::<T>()`.
/// - `data.as_ptr()` must be aligned for `T`.
pub fn extract<T: FullyOccupied>(bytes: &[u8]) -> &T {
    // SAFETY: The unsafe impl of `FullyOccupied` promises that the bytes are valid for T.
    unsafe { extract_unchecked(bytes) }
}

/// Reinteprets a byte slice as a slice of T.
///
/// # Panics
///
/// Panics if any of the following preconditions fail:
///
/// - `data.len()` must be a multiple of `size_of::<T>()`.
/// - `data.as_ptr()` must be aligned for `T`.
pub fn extract_slice<T: FullyOccupied>(bytes: &[u8]) -> &[T] {
    // SAFETY: The unsafe impl of `FullyOccupied` promises that the bytes are valid for T.
    unsafe { extract_slice_unchecked(bytes) }
}

/// Reinteprets a prefix of a byte slice as a value of T, regardless of whether T is fully occupied.
///
/// # Safety
///
/// The bytes must be valid for the given type.
///
/// # Panics
///
/// Panics if any of the following preconditions fail:
///
/// - `data.len()` must be at least `size_of::<T>()`.
/// - `data.as_ptr()` must be aligned for `T`.
pub unsafe fn extract_unchecked<T>(bytes: &[u8]) -> &T {
    let bytes = &bytes[..size_of::<T>()];
    let ptr = bytes.as_ptr() as *const T;
    assert!(is_aligned(ptr));

    // SAFETY: The caller asserts these bytes are valid for T. The memory is from the provided byte
    // slice and thus is valid. Alignment has been checked.
    unsafe { &*ptr }
}

/// Reinterprets a byte slice as a slice of T, regardless of whether T is fully occupied.
///
/// # Safety
///
/// The entire byte slice must be valid for a sequence of the given type.
///
/// # Panics
///
/// Panics if any of the following preconditions fail:
///
/// - `data.len()` must be a multiple of `size_of::<T>()`.
/// - `data.as_ptr()` must be aligned for `T`.
pub unsafe fn extract_slice_unchecked<T>(bytes: &[u8]) -> &[T] {
    assert_eq!(bytes.len() % size_of::<T>(), 0);
    let ptr = bytes.as_ptr() as *const T;
    assert!(is_aligned(ptr));

    // SAFETY: The caller asserts these bytes are valid for T. The memory is from the provided byte
    // slice and thus is valid. Alignment has been checked.
    unsafe { from_raw_parts(ptr, bytes.len() / size_of::<T>()) }
}

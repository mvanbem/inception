pub fn is_aligned<T>(ptr: *const T) -> bool {
    (ptr as usize) % std::mem::align_of::<T>() == 0
}

// SAFETY: The bytes at offset must be valid for the given type.
pub unsafe fn extract_at<T>(data: &[u8], offset: usize) -> &T {
    let offset_end = offset + std::mem::size_of::<T>();
    let bytes = &data[offset..offset_end];

    let ptr = bytes.as_ptr() as *const T;
    assert!(is_aligned(ptr));

    unsafe { std::mem::transmute(ptr) }
}

// SAFETY: The entire byte slice must be valid for a sequence of the given type.
pub unsafe fn extract_slice<T>(data: &[u8]) -> &[T] {
    assert_eq!(data.len() % std::mem::size_of::<T>(), 0);

    let ptr = data.as_ptr() as *const T;
    assert!(is_aligned(ptr));

    unsafe { std::slice::from_raw_parts(ptr, data.len() / std::mem::size_of::<T>()) }
}

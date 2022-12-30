#[derive(Clone, Copy)]
pub struct PermissionRoot {
    _private: (),
}

impl PermissionRoot {
    /// # Safety
    ///
    /// This function is marked unsafe to make calls noisy. Call it only from the beginning of
    /// `main` or an interrupt handler to establish a permission root.
    pub unsafe fn new_unchecked() -> Self {
        Self { _private: () }
    }
}

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

/// This is like a `RefCell<Option<T>>`, except `Sync` and with one word of overhead.
pub struct DriverState<T> {
    flags: AtomicU8,
    inner: UnsafeCell<MaybeUninit<T>>,
}

#[repr(u8)]
enum State {
    Uninit,
    Available,
    Locked,
}

impl<T> DriverState<T> {
    pub const fn uninit() -> Self {
        Self {
            flags: AtomicU8::new(State::Uninit as u8),
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn init_with(&self, f: impl FnOnce() -> T) {
        // Atomically claim the uninitialized state.
        self.flags
            .compare_exchange(
                State::Uninit as u8,
                State::Locked as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .unwrap();

        // SAFETY: We have the lock.
        let state = unsafe { &mut *self.inner.get() };
        state.write(f());

        // Atomically unlock the now-initialized state.
        self.flags
            .compare_exchange(
                State::Locked as u8,
                State::Available as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .unwrap();
    }

    pub fn with_state(&self, f: impl FnOnce(&mut T)) {
        // Atomically lock the state.
        self.flags
            .compare_exchange(
                State::Available as u8,
                State::Locked as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .unwrap();

        // SAFETY: We have the lock.
        f(unsafe { (*self.inner.get()).assume_init_mut() });

        // Atomically unlock the state.
        self.flags
            .compare_exchange(
                State::Locked as u8,
                State::Available as u8,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .unwrap();
    }
}

unsafe impl<T> Sync for DriverState<T> {}

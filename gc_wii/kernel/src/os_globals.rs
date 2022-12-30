use core::marker::PhantomData;
use core::ptr;

#[repr(C)]
struct Data {
    vi_interrupt_fired: u8,
}

pub struct OsGlobals<'data> {
    _phantom_data: PhantomData<&'data Data>,
}

impl<'data> OsGlobals<'data> {
    const PTR: *mut Data = 0x80000000usize as _;

    /// # Safety
    ///
    /// All calls must have disjoint lifetimes.
    pub unsafe fn new_unchecked() -> Self {
        Self {
            _phantom_data: PhantomData,
        }
    }

    pub fn read_vi_interrupt_fired(&self) -> bool {
        let value = unsafe { ptr::read_volatile(&(*Self::PTR).vi_interrupt_fired) };
        value != 0
    }

    pub fn write_vi_interrupt_fired(&self, value: bool) {
        unsafe { ptr::write_volatile(&mut (*Self::PTR).vi_interrupt_fired, value as u8) };
    }
}

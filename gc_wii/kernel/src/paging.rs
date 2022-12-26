#![allow(dead_code)]

use mvbitfield::prelude::*;

#[repr(C)]
pub struct PageTableEntry {
    pub a: PageTableEntryA,
    pub b: PageTableEntryB,
}

mvbitfield!(
    pub struct PageTableEntryA: u32 {
        pub abbreviated_page_index: 6,
        pub hash_function_identifier: 1 as bool,
        pub virtual_segment_id: 24,
        pub valid: 1 as bool,
    }
);

mvbitfield! {
    pub struct PageTableEntryB: u32 {
        pub page_protection_bits: 2,
        _reserved: 1,
        pub wimg: 4 as Wimg,
        pub changed: 1 as bool,
        pub referenced: 1 as bool,
        _reserved: 3,
        pub physical_page_number: 20,
    }
}

mvbitfield! {
    pub struct Wimg: U4 {
        pub guarded: 1 as bool,
        pub memory_coherence: 1 as bool,
        pub caching_inhibited: 1 as bool,
        pub write_through: 1 as bool,
    }
}

pub fn do_stuff() {
    // arbitrarily 1 MiB into memory
    let page_table: *mut PageTableEntry = 0x80100000usize as _;

    const PAGE_TABLE_ENTRY: PageTableEntry = PageTableEntry {
        a: PageTableEntryA::zero()
            .with_abbreviated_page_index(U6::new_masked(0))
            .with_hash_function_identifier(true)
            .with_virtual_segment_id(U24::new_masked(0))
            .with_valid(true),
        b: PageTableEntryB::zero()
            .with_page_protection_bits(U2::new_masked(0))
            .with_wimg(
                Wimg::zero()
                    .with_write_through(false)
                    .with_caching_inhibited(false)
                    .with_memory_coherence(false)
                    .with_guarded(false),
            )
            .with_changed(true)
            .with_referenced(true)
            .with_physical_page_number(U20::new_masked(0)),
    };

    unsafe {
        *page_table = PAGE_TABLE_ENTRY;
    }
}

use core::sync::atomic::AtomicBool;

pub struct OsGlobals {
    pub vi_interrupt_fired: AtomicBool,
}

pub static OS_GLOBALS: OsGlobals = OsGlobals{
    vi_interrupt_fired: AtomicBool::new(false),
};

use core::ffi::c_void;
use core::mem::MaybeUninit;

use crate::app_loader::{AppLoader, LoadCommand};

pub type OsReportFn = unsafe extern "C" fn(msg: *const u8, ...);
pub const OS_BOOT_INFO2: *mut *mut c_void = 0x800000f4usize as _;
pub const OS_BOOT_INFO2_SIZE: usize = 0x2000;
pub const OS_FST_ADDRESS: *mut *mut c_void = 0x80000038usize as _;
pub const OS_FST_SIZE: *mut usize = 0x8000003cusize as _;

type InitFn = unsafe extern "C" fn(os_report: OsReportFn);
type MainFn = unsafe extern "C" fn(
    addr_ptr: *mut *mut c_void,
    len_ptr: *mut usize,
    offset_ptr: *mut usize,
) -> bool;
type CloseFn = unsafe extern "C" fn() -> *const c_void;

#[link_section = ".apploader.entry"]
#[no_mangle]
pub unsafe extern "C" fn apploader_entry(
    init_fn: *mut InitFn,
    main_fn: *mut MainFn,
    close_fn: *mut CloseFn,
) {
    *init_fn = init;
    *main_fn = main;
    *close_fn = close;
}

static mut APP_LOADER: MaybeUninit<AppLoader> = MaybeUninit::uninit();

unsafe extern "C" fn init(os_report: OsReportFn) {
    APP_LOADER.write(AppLoader::new(os_report));
}

unsafe extern "C" fn main(
    addr_ptr: *mut *mut c_void,
    len_ptr: *mut usize,
    offset_ptr: *mut usize,
) -> bool {
    match APP_LOADER.assume_init_mut().main() {
        Some(LoadCommand { addr, len, offset }) => {
            *addr_ptr = addr;
            *len_ptr = len;
            *offset_ptr = offset;
            true
        }
        None => false,
    }
}

unsafe extern "C" fn close() -> *const c_void {
    let entry_point = APP_LOADER.assume_init_ref().entry_point();
    APP_LOADER.assume_init_drop();
    entry_point
}

use core::arch::asm;

use array_lit::arr;
use gamecube_cpu::registers::msr::{MachineState, PrivilegeLevel};
use mvbitfield::prelude::*;

use crate::driver;
use crate::driver::timer::Timestamp;

extern "C" {
    fn call_thread_scheduler();
    /// The caller MUST load SPRG3 with an interrupt bitfield indicating any interrupts that have
    /// occurred.
    fn thread_scheduler() -> !;
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Thread {
    state: ThreadState,
    waiting_for: WaitingFor,
    suspended_state: ThreadSuspendedState,
    exception_save: [u32; 18],
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum ThreadState {
    Invalid,
    Waiting,
    Runnable,
}

mvbitfield! {
    pub struct WaitingFor: u32 {
        pub video_interface: 4 as VideoInterfaceInterrupt,
        pub timer: 1 as bool,
    }
}

mvbitfield! {
    pub struct VideoInterfaceInterrupt: U4 {
        pub display_interrupt_0: 1 as bool,
        pub display_interrupt_1: 1 as bool,
        pub display_interrupt_2: 1 as bool,
        pub display_interrupt_3: 1 as bool,
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ThreadSuspendedState {
    nia: u32,
    msr: u32,
    ctr: u32,
    xer: u32,
    lr: u32,
    cr: u32,
    gpr: [u32; 32],
}

impl ThreadSuspendedState {
    const fn zero() -> Self {
        Self {
            nia: 0,
            msr: 0,
            ctr: 0,
            xer: 0,
            lr: 0,
            cr: 0,
            gpr: [0; 32],
        }
    }
}

const MAX_THREAD_COUNT: usize = 8;
#[no_mangle]
pub static mut THREAD_TABLE: [Thread; MAX_THREAD_COUNT] = [Thread {
    state: ThreadState::Invalid,
    waiting_for: WaitingFor::zero(),
    suspended_state: ThreadSuspendedState::zero(),
    exception_save: [0; 18],
}; MAX_THREAD_COUNT];
#[no_mangle]
pub static mut THREAD_COUNT: usize = 0;
#[no_mangle]
pub static mut CURRENT_THREAD: ThreadId = ThreadId::NONE;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ThreadId(pub usize);

impl ThreadId {
    pub const NONE: Self = Self(!0);

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }

    pub fn is_some(self) -> bool {
        !self.is_none()
    }
}

pub const USER_MACHINE_STATE: MachineState = MachineState::zero()
    .with_exception_is_recoverable(true)
    .with_data_address_translation_enabled(true)
    .with_instruction_address_translation_enabled(true)
    .with_machine_check_enabled(true)
    .with_privilege_level(PrivilegeLevel::User)
    .with_external_interrupts_enabled(true);

pub fn create_thread(
    entry: extern "C" fn() -> !,
    msr: MachineState,
    waiting_for: Option<WaitingFor>,
) -> ThreadId {
    let id = unsafe { THREAD_COUNT };
    if id >= MAX_THREAD_COUNT {
        panic!("out of threads");
    }

    let thread = unsafe { &mut THREAD_TABLE[id] };
    *thread = Thread {
        state: if waiting_for.is_some() {
            ThreadState::Waiting
        } else {
            ThreadState::Runnable
        },
        waiting_for: waiting_for.unwrap_or(WaitingFor::zero()),
        suspended_state: ThreadSuspendedState {
            nia: entry as u32,
            msr: msr.as_u32(),
            gpr: arr![0; 32; {
                // Thread 0's stack is at the top of memory. Each subsequent thread is 1 MiB lower.
                [1]: [0x817ffff0 - (id as u32) * 0x00100000],
            }],
            ..ThreadSuspendedState::zero()
        },
        exception_save: [0; 18],
    };
    unsafe { THREAD_COUNT += 1 };
    ThreadId(id)
}

pub unsafe fn enter_threading() -> ! {
    assert!(unsafe { CURRENT_THREAD }.is_none());
    asm!(
        "li %r3,0",
        "mtsprg3 %r3",
        out("r3") _,
    );
    thread_scheduler()
}

pub fn suspend_current_thread(waiting_for: WaitingFor) {
    // Update this thread's state in the thread table.
    let id = unsafe { CURRENT_THREAD };
    assert!(id.is_some());
    let thread = unsafe { &mut THREAD_TABLE[id.0] };
    thread.state = ThreadState::Waiting;
    thread.waiting_for = waiting_for;

    // Enter the scheduler. We will resume from here.
    unsafe { call_thread_scheduler() };
}

pub fn sleep_current_thread_until(timestamp: Timestamp) {
    driver::timer::insert(timestamp, unsafe { CURRENT_THREAD });
    suspend_current_thread(WaitingFor::zero().with_timer(true));
}

/// Wakes the indicated thread if it is waiting for any of the bits set in the given wake mask.
pub fn wake_thread(thread_id: ThreadId, wake_mask: WaitingFor) {
    let thread = unsafe { &mut THREAD_TABLE[thread_id.0] };
    if thread.state == ThreadState::Waiting && thread.waiting_for.as_u32() & wake_mask.as_u32() != 0
    {
        thread.state = ThreadState::Runnable;
        thread.waiting_for = WaitingFor::zero();
    }
}

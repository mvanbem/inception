use core::arch::asm;

use array_lit::arr;
use gamecube_cpu::registers::msr::{MachineState, PrivilegeLevel};

use crate::external_interrupt::Interrupt;

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
    waiting_for: Interrupt,
    suspended_state: ThreadSuspendedState,
    exception_save: [u32; 18],
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum ThreadState {
    Invalid,
    Waiting,
    Runnable,
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

const MAX_THREAD_COUNT: usize = 2;
#[no_mangle]
pub static mut THREAD_TABLE: [Thread; MAX_THREAD_COUNT] = [Thread {
    state: ThreadState::Invalid,
    waiting_for: Interrupt::zero(),
    suspended_state: ThreadSuspendedState::zero(),
    exception_save: [0; 18],
}; MAX_THREAD_COUNT];
#[no_mangle]
pub static mut THREAD_COUNT: usize = 0;
#[no_mangle]
pub static mut CURRENT_THREAD: usize = !0;

#[derive(Clone, Copy)]
pub struct ThreadId(pub usize);

pub const KERNEL_MACHINE_STATE: MachineState = MachineState::zero()
    .with_exception_is_recoverable(true)
    .with_data_address_translation_enabled(true)
    .with_instruction_address_translation_enabled(true)
    .with_machine_check_enabled(true)
    .with_privilege_level(PrivilegeLevel::Supervisor)
    .with_external_interrupts_enabled(true);

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
    waiting_for: Option<Interrupt>,
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
        waiting_for: waiting_for.unwrap_or(Interrupt::zero()),
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
    assert_eq!(unsafe { CURRENT_THREAD }, !0);
    asm!(
        "li %r3,0",
        "mtsprg3 %r3",
        out("r3") _,
    );
    thread_scheduler()
}

pub fn suspend_current_thread(waiting_for: Interrupt) {
    // Update this thread's state in the thread table.
    let id = unsafe { CURRENT_THREAD };
    assert_ne!(id, !0);
    let thread = unsafe { &mut THREAD_TABLE[id] };
    thread.state = ThreadState::Waiting;
    thread.waiting_for = waiting_for;

    // Enter the scheduler. We will resume from here.
    unsafe { call_thread_scheduler() };
}

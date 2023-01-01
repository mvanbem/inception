use arrayvec::ArrayVec;
use gamecube_cpu::registers::{set_decrementer, time_base};

use crate::driver::driver_state::DriverState;
use crate::thread::ThreadId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

const MAX_ENTRIES: usize = 8;

struct Timer {
    entries: ArrayVec<Entry, MAX_ENTRIES>,
}

struct Entry {
    timestamp: Timestamp,
    thread_id: ThreadId,
}

#[link_section = ".bss"]
static STATE: DriverState<Timer> = DriverState::uninit();

pub fn init() {
    STATE.init_with(|| Timer {
        entries: ArrayVec::new_const(),
    });
}

pub fn insert(timestamp: Timestamp, thread_id: ThreadId) {
    STATE.with_state(|timer| {
        let now = time_base();

        // Add the new timestamp to the sorted list of sleep timestamps.
        timer.entries.push(Entry {
            timestamp,
            thread_id,
        });
        timer.entries.sort_unstable_by_key(|entry| entry.timestamp);

        // Configure the decrementer for the earliest requested timestamp.
        let ticks_remaining =
            u32::try_from(timer.entries[0].timestamp.0.saturating_sub(now)).unwrap_or(u32::MAX);
        set_decrementer(ticks_remaining);
    });
}

pub fn for_each_elapsed(mut f: impl FnMut(ThreadId)) {
    STATE.with_state(|timer| {
        let now = Timestamp(time_base());

        // Scan and compact the list of sleep timestamps.
        timer.entries.retain(|entry| {
            let elapsed = entry.timestamp <= now;
            if elapsed {
                f(entry.thread_id);
            }
            !elapsed
        });
    });
}

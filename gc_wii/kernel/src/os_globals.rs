use core::sync::atomic::AtomicBool;

use crate::text_console::TextConsole;

pub static VI_INTERRUPT_FIRED: AtomicBool = AtomicBool::new(false);

pub static mut TEXT_CONSOLE: TextConsole = TextConsole::new();

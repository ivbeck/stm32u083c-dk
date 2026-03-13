use core::sync::atomic::AtomicU32;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use crate::drivers::lcd::LcdCommand;

pub static DELAY_MS: AtomicU32 = AtomicU32::new(100);
pub static LCD_CMD: Signal<CriticalSectionRawMutex, LcdCommand> = Signal::new();

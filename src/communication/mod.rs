use core::sync::atomic::AtomicU32;

use crate::drivers::lcd::{LcdChannel, LcdMessage};

pub static DELAY_MS: AtomicU32 = AtomicU32::new(100);
pub static LCD_QUEUE: LcdChannel = LcdChannel::new();

/// Send a message to the LCD queue, dropping the oldest entry if full
/// so the latest message always gets through.
pub fn lcd_send(msg: LcdMessage) {
    if let Err(embassy_sync::channel::TrySendError::Full(msg)) = LCD_QUEUE.try_send(msg) {
        let _ = LCD_QUEUE.try_receive();
        let _ = LCD_QUEUE.try_send(msg);
    }
}

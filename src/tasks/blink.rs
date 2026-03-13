use core::sync::atomic::Ordering;

use defmt::info;

use crate::{communication::DELAY_MS, drivers::dedicated_rgb_leds::Rgb};

#[embassy_executor::task]
pub async fn blink_task(mut rgb: Rgb) {
    info!("Starting blink on STM32U083...");

    loop {
        let delay_ms = DELAY_MS.load(Ordering::Relaxed);
        rgb.blink_cascade(u64::from(delay_ms)).await;
    }
}

use core::sync::atomic::Ordering;

use defmt::info;
use embassy_stm32::{adc::AnyAdcChannel, peripherals::ADC1};
use embassy_time::Timer;

use crate::{
    communication::{DELAY_MS, lcd_send},
    drivers::{
        joystick::{JoyDirection, Joystick},
        lcd::LcdMessage,
    },
    format_str,
};

#[embassy_executor::task]
pub async fn joystick_task(mut joystick: Joystick<AnyAdcChannel<'static, ADC1>>) {
    const DELAY_MAX_MS: u32 = 5000;
    const DELAY_STEP_MS: u32 = 50;

    let mut prev: Option<JoyDirection> = None;
    loop {
        let dir = joystick.read();

        if prev.is_none() || dir != prev.expect("Branch evaluated") {
            match dir {
                JoyDirection::Up => {
                    let v = DELAY_MS.load(Ordering::Relaxed);
                    let new_v = v.saturating_sub(DELAY_STEP_MS);
                    DELAY_MS.store(new_v, Ordering::Relaxed);
                    info!("Speed up: {}ms", new_v);
                    lcd_send(LcdMessage::text(
                        format_str!("UP to {}ms", new_v).as_str(),
                        200,
                    ));
                }
                JoyDirection::Down => {
                    let v = DELAY_MS.load(Ordering::Relaxed);
                    let new_v = v.saturating_add(DELAY_STEP_MS).min(DELAY_MAX_MS);
                    DELAY_MS.store(new_v, Ordering::Relaxed);
                    info!("DOWN: {}ms", new_v);
                    lcd_send(LcdMessage::text(
                        format_str!("DOWN to {}ms", new_v).as_str(),
                        200,
                    ));
                }
                JoyDirection::Right => {
                    lcd_send(LcdMessage::text("Wassup Dawg", 200));
                }
                JoyDirection::Left => {
                    lcd_send(LcdMessage::Clear);
                }
                _ => {}
            }
            prev = Some(dir);
        }

        Timer::after_millis(20).await;
    }
}

#![no_main]
#![no_std]

use stm32u083c_dk as _; // memory layout + panic handler

mod drivers;
mod macros;

use defmt::*;

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_executor::Spawner;
use embassy_stm32::adc::{AdcChannel, AnyAdcChannel};
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::rcc::LsConfig;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

use drivers::dedicated_rgb_leds::Rgb;
use drivers::joystick::{JoyDirection, Joystick};
use drivers::lcd::{LcdCommand, SegLcd};

const DELAY_MAX_MS: u32 = 5000;
const DELAY_STEP_MS: u32 = 50;

static DELAY_MS: AtomicU32 = AtomicU32::new(100);
static LCD_CMD: Signal<CriticalSectionRawMutex, LcdCommand> = Signal::new();

#[embassy_executor::task]
async fn lcd_task(mut lcd: SegLcd) {
    lcd.run(&LCD_CMD).await
}

#[embassy_executor::task]
async fn joystick_task(mut joystick: Joystick<AnyAdcChannel<'static, ADC1>>) {
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
                    LCD_CMD.signal(LcdCommand::scroll(
                        format_str!("UP to {}ms", new_v).as_str(),
                        200,
                    ));
                }
                JoyDirection::Down => {
                    let v = DELAY_MS.load(Ordering::Relaxed);
                    let new_v = v.saturating_add(DELAY_STEP_MS).min(DELAY_MAX_MS);
                    DELAY_MS.store(new_v, Ordering::Relaxed);
                    info!("DOWN: {}ms", new_v);
                    LCD_CMD.signal(LcdCommand::scroll(
                        format_str!("DOWN to {}ms", new_v).as_str(),
                        200,
                    ));
                }
                JoyDirection::Right => {
                    LCD_CMD.signal(LcdCommand::scroll_loop("Wassup Dawg", 200));
                }
                JoyDirection::Left => {
                    LCD_CMD.signal(LcdCommand::Clear);
                }
                _ => {}
            }
            prev = Some(dir);
        }

        Timer::after_millis(20).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    config.rcc.ls = LsConfig::default_lse();
    let p = embassy_stm32::init(config);

    let rgb = Rgb::new(p.PB2, p.PC13, p.PA5);

    // SAFETY: called once before any LCD pins are used elsewhere.
    let seg_lcd = unsafe { SegLcd::from_peripherals() };

    let joystick = Joystick::new(p.ADC1, p.PC2.degrade_adc());

    spawner.spawn(lcd_task(seg_lcd)).unwrap();
    spawner.spawn(joystick_task(joystick)).unwrap();
    spawner.spawn(blink_task(rgb)).unwrap();

    LCD_CMD.signal(LcdCommand::number(DELAY_MS.load(Ordering::Relaxed)));

    loop {
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::task]
async fn blink_task(mut rgb: Rgb) {
    info!("Starting blink on STM32U083...");

    loop {
        let delay_ms = DELAY_MS.load(Ordering::Relaxed);
        rgb.blink_cascade(delay_ms as u64).await;
    }
}

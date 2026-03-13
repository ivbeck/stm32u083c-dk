#![no_main]
#![no_std]
#![allow(unreachable_code)]

use stm32u083c_dk as _; // memory layout + panic handler

mod drivers;
use defmt::*;

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use embassy_executor::Spawner;
use embassy_stm32::adc::{AdcChannel, AnyAdcChannel};
use embassy_stm32::peripherals::ADC1;
use embassy_stm32::rcc::LsConfig;
use embassy_time::Timer;

use drivers::dedicated_rgb_leds::Rgb;
use drivers::joystick::{JoyDirection, Joystick};
use drivers::lcd::SegLcd;

const DELAY_MAX_MS: u32 = 5000;
const DELAY_STEP_MS: u32 = 50;

static DELAY_MS: AtomicU32 = AtomicU32::new(100);
static SHOW_NEXT: AtomicBool = AtomicBool::new(false);
static SHOW_PREV: AtomicBool = AtomicBool::new(false);

#[embassy_executor::task]
async fn joystick_task(mut joystick: Joystick<AnyAdcChannel<'static, ADC1>>, mut seg_lcd: SegLcd) {
    let mut prev: Option<JoyDirection> = None;
    loop {
        let dir = joystick.read();

        // Only fire on rising edge (new press, not held)
        if prev.is_none() || dir != prev.expect("Branch evaluated") {
            match dir {
                JoyDirection::Up => {
                    let v = DELAY_MS.load(Ordering::Relaxed);
                    DELAY_MS.store(v.saturating_sub(DELAY_STEP_MS), Ordering::Relaxed);
                    info!("Speed up: {}ms", DELAY_MS.load(Ordering::Relaxed));
                    seg_lcd.display_number(DELAY_MS.load(Ordering::Relaxed));
                }
                JoyDirection::Down => {
                    let v = DELAY_MS.load(Ordering::Relaxed);
                    DELAY_MS.store(
                        v.saturating_add(DELAY_STEP_MS).min(DELAY_MAX_MS),
                        Ordering::Relaxed,
                    );
                    info!("Speed down: {}ms", DELAY_MS.load(Ordering::Relaxed));
                    seg_lcd.display_number(DELAY_MS.load(Ordering::Relaxed));
                }
                JoyDirection::Right => {
                    SHOW_NEXT.store(true, Ordering::Relaxed);
                }
                JoyDirection::Left => {
                    SHOW_PREV.store(true, Ordering::Relaxed);
                }
                _ => {}
            }
            prev = Some(dir);
        }

        Timer::after_millis(20).await; // poll joystick at 50 Hz
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // LSE is required as the LCD clock source
    let mut config = embassy_stm32::Config::default();
    config.rcc.ls = LsConfig::default_lse();
    let p = embassy_stm32::init(config);

    let rgb = Rgb::new(p.PB2, p.PC13, p.PA5);

    // SAFETY: called once before any LCD pins are used elsewhere.
    let seg_lcd = unsafe { SegLcd::from_peripherals() };

    // degrade_adc() erases pin type so Joystick can be passed to a task
    let joystick = Joystick::new(p.ADC1, p.PC2.degrade_adc());
    spawner.spawn(joystick_task(joystick, seg_lcd)).unwrap();

    // spawner.spawn(test_segments(seg_lcd)).unwrap();
    spawner.spawn(blink_task(rgb)).unwrap();

    loop {
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::task]
async fn test_segments(mut seg_lcd: SegLcd) {
    info!("Testing LCD: displaying 123456...");
    seg_lcd.display_number(123456);
    Timer::after_millis(3000).await;

    info!("Entering single-segment scan mode (joystick left/right to navigate)");
    const MAX_GLASS_SEG: u8 = 24;

    let mut glass_seg: u8 = 0;
    let mut com: u8 = 0;

    loop {
        if SHOW_NEXT.load(Ordering::Relaxed) {
            if glass_seg < MAX_GLASS_SEG - 1 {
                glass_seg += 1;
            } else {
                glass_seg = 0;
                com = (com + 1) % 4;
            }
            info!("Glass SEG {} on COM{}", glass_seg, com);
        }
        if SHOW_PREV.load(Ordering::Relaxed) {
            if glass_seg > 0 {
                glass_seg -= 1;
            } else {
                glass_seg = MAX_GLASS_SEG - 1;
                com = if com > 0 { com - 1 } else { 3 };
            }
            info!("Glass SEG {} on COM{}", glass_seg, com);
        }

        seg_lcd.test_single_segment(com, glass_seg);

        SHOW_NEXT.store(false, Ordering::Relaxed);
        SHOW_PREV.store(false, Ordering::Relaxed);
        Timer::after_millis(100).await;
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

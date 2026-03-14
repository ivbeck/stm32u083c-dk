#![no_main]
#![no_std]

use defmt_rtt as _;
use lib_stm32u083c_dk::drivers::dedicated_rgb_leds::Rgb;
use lib_stm32u083c_dk::drivers::joystick::Joystick;
use lib_stm32u083c_dk::drivers::lcd::SegLcd;
use lib_stm32u083c_dk::drivers::temp_sensor::Stts22h;
use lib_stm32u083c_dk::tasks::{blink_task, joystick_task, lcd_task, temp_sensor_task};
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::adc::AdcChannel;
use embassy_stm32::rcc::LsConfig;
use embassy_time::Timer;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    config.rcc.ls = LsConfig::default_lse();
    let p = embassy_stm32::init(config);

    let rgb = Rgb::new(p.PB2, p.PC13, p.PA5);

    // SAFETY: called once before any LCD pins are used elsewhere.
    let seg_lcd = unsafe { SegLcd::from_peripherals() };
    let joystick = Joystick::new(p.ADC1, p.PC2.degrade_adc());

    spawner.spawn(blink_task(rgb)).expect("blink_task");
    spawner.spawn(lcd_task(seg_lcd)).expect("lcd_task");
    spawner
        .spawn(joystick_task(joystick))
        .expect("joystick_task");

    let sensor = match Stts22h::new(p.I2C1, p.PB8, p.PB7) {
        Ok(sensor) => sensor,
        Err(err) => {
            defmt::error!("STTS22H init failed: {}", err);
            return;
        }
    };

    spawner
        .spawn(temp_sensor_task(sensor, false))
        .expect("temp_sensor_task");

    loop {
        Timer::after_millis(1000).await;
    }
}

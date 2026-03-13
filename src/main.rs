#![no_main]
#![no_std]

use stm32u083c_dk as _;
use stm32u083c_dk::communication::lcd_send;
use stm32u083c_dk::drivers::dedicated_rgb_leds::Rgb;
use stm32u083c_dk::drivers::joystick::Joystick;
use stm32u083c_dk::drivers::lcd::{LcdMessage, SegLcd};
use stm32u083c_dk::drivers::temp_sensor::Stts22h;
use stm32u083c_dk::tasks::{blink_task, joystick_task, lcd_task, temp_sensor_task};

mod macros;

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

    spawner.spawn(lcd_task(seg_lcd)).expect("lcd_task");
    spawner
        .spawn(joystick_task(joystick))
        .expect("joystick_task");
    spawner.spawn(blink_task(rgb)).expect("blink_task");

    match Stts22h::new(p.I2C1, p.PB8, p.PB7) {
        Ok(sensor) => {
            spawner
                .spawn(temp_sensor_task(sensor))
                .expect("temp_sensor_task");
        }
        Err(err) => {
            defmt::error!("STTS22H init failed: {}", err);
        }
    }

    lcd_send(LcdMessage::text("Hi!", 200));

    loop {
        Timer::after_millis(1000).await;
    }
}

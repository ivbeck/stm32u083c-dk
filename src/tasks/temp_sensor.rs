use core::f32;

use defmt::{info, warn};
use embassy_time::Timer;

use crate::{
    communication::lcd_send,
    drivers::{lcd::LcdMessage, temp_sensor::Stts22h},
    format_str,
};

#[embassy_executor::task]
pub async fn temp_sensor_task(mut sensor: Stts22h) {
    let mut last_temp = f32::NEG_INFINITY;

    loop {
        match sensor.read_temperature() {
            Ok(temp) => {
                if f32_abs_diff(temp, last_temp) > 0.5 {
                    last_temp = temp;
                    info!("Board temperature: {}°C", temp);
                    lcd_send(LcdMessage::text(
                        format_str!("Temp: {}C", temp).as_str(),
                        200,
                    ));
                }
            }
            Err(_) => {
                warn!("STTS22H read failed");
            }
        }

        Timer::after_millis(2000).await;
    }
}

fn f32_abs_diff(a: f32, b: f32) -> f32 {
    if a > b { a - b } else { b - a }
}

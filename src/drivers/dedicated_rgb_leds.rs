use embassy_stm32::{
    Peri,
    gpio::{Level, Output, Pin, Speed},
};
use embassy_time::Timer;

pub struct Rgb {
    red: Output<'static>,
    green: Output<'static>,
    blue: Output<'static>,
}

impl Rgb {
    pub fn new(
        red_pin: Peri<'static, impl Pin>,
        green_pin: Peri<'static, impl Pin>,
        blue_pin: Peri<'static, impl Pin>,
    ) -> Self {
        Self {
            red: Output::new(red_pin, Level::Low, Speed::Low),
            green: Output::new(green_pin, Level::Low, Speed::Low),
            blue: Output::new(blue_pin, Level::Low, Speed::Low),
        }
    }

    pub async fn blink_cascade(&mut self, delay_ms: u64) {
        self.green.set_high();
        Timer::after_millis(delay_ms).await;
        self.blue.set_high();
        Timer::after_millis(delay_ms).await;
        self.red.set_high();

        Timer::after_millis(delay_ms).await;
        self.green.set_low();
        Timer::after_millis(delay_ms).await;
        self.blue.set_low();
        Timer::after_millis(delay_ms).await;
        self.red.set_low();
        Timer::after_millis(delay_ms).await;
    }
}

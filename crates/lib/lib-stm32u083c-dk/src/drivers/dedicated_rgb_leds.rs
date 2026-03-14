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

    /// Knight Rider / Larson scanner: one LED sweeps back and forth (G → B → R → B → G …).
    pub async fn larson_scanner(&mut self, delay_ms: u64, cycles: u32) {
        #[derive(Clone, Copy)]
        enum Led {
            Red,
            Green,
            Blue,
        }
        let set = |s: &mut Self, led: Led, on: bool| {
            let level = if on { Level::High } else { Level::Low };
            match led {
                Led::Red => s.red.set_level(level),
                Led::Green => s.green.set_level(level),
                Led::Blue => s.blue.set_level(level),
            }
        };
        // Physical layout (top→bottom): Green, Blue, Red.
        // Sweep in that order, then back.
        let order = [Led::Green, Led::Blue, Led::Red];
        for _ in 0..cycles {
            for &led in &order {
                set(self, led, true);
                Timer::after_millis(delay_ms).await;
                set(self, led, false);
            }
            for &led in order.iter().rev() {
                set(self, led, true);
                Timer::after_millis(delay_ms).await;
                set(self, led, false);
            }
        }
    }

    /// Snake: two adjacent LEDs chase top-to-bottom (GB → BR → RG → GB …).
    pub async fn snake(&mut self, delay_ms: u64, cycles: u32) {
        // Start with top two LEDs: Green + Blue.
        self.green.set_high();
        self.blue.set_high();
        for _ in 0..cycles {
            Timer::after_millis(delay_ms).await;
            // Middle + bottom: Blue + Red.
            self.green.set_low();
            self.red.set_high();
            Timer::after_millis(delay_ms).await;
            // Bottom + top: Red + Green.
            self.blue.set_low();
            self.green.set_high();
            Timer::after_millis(delay_ms).await;
            // Back to top + middle: Green + Blue.
            self.red.set_low();
            self.blue.set_high();
        }
        self.red.set_low();
        self.green.set_low();
        self.blue.set_low();
    }

    /// Binary count: LEDs show 0..7 in binary (R=lsb, G, B=msb), then repeat.
    pub async fn binary_count(&mut self, delay_ms: u64, cycles: u32) {
        for _ in 0..cycles {
            for n in 0..8u8 {
                self.red
                    .set_level(if n & 1 != 0 { Level::High } else { Level::Low });
                self.green
                    .set_level(if n & 2 != 0 { Level::High } else { Level::Low });
                self.blue
                    .set_level(if n & 4 != 0 { Level::High } else { Level::Low });
                Timer::after_millis(delay_ms).await;
            }
        }
        self.red.set_low();
        self.green.set_low();
        self.blue.set_low();
    }

    /// Full animation loop: larson → snake → binary count, then repeat.
    pub async fn animation_loop(&mut self, delay_ms: u64) {
        self.larson_scanner(delay_ms, 5).await;
        self.snake(delay_ms, 3).await;
        self.binary_count(delay_ms, 1).await;
    }
}

use core::fmt::Display;

use defmt::{dbg, debug};
use embassy_stm32::{
    Peri,
    adc::{Adc, AdcChannel, SampleTime},
    peripherals::ADC1,
};

// Log raw values to calibrate: defmt::debug!("{}", value)
const CENTER_THRESHOLD_MIN: u16 = 0;
const CENTER_THRESHOLD_MAX: u16 = 300;

const LEFT_THRESHOLD_MIN: u16 = 600;
const LEFT_THRESHOLD_MAX: u16 = 1000;

const DOWN_THRESHOLD_MIN: u16 = 1400;
const DOWN_THRESHOLD_MAX: u16 = 1800;

const UP_THRESHOLD_MIN: u16 = 2200;
const UP_THRESHOLD_MAX: u16 = 2600;

const RIGHT_THRESHOLD_MIN: u16 = 3000;
const RIGHT_THRESHOLD_MAX: u16 = 3400;

const NEUTRAL_THRESHOLD_MIN: u16 = 3800;
const NEUTRAL_THRESHOLD_MAX: u16 = 4200;

#[derive(Clone, Copy, PartialEq)]
pub enum JoyDirection {
    Up,
    Down,
    Left,
    Right,
    Center,
    Neutral,
    Invalid,
}

impl defmt::Format for JoyDirection {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            JoyDirection::Up => defmt::write!(fmt, "Up"),
            JoyDirection::Down => defmt::write!(fmt, "Down"),
            JoyDirection::Left => defmt::write!(fmt, "Left"),
            JoyDirection::Right => defmt::write!(fmt, "Right"),
            JoyDirection::Center => defmt::write!(fmt, "Center"),
            JoyDirection::Neutral => defmt::write!(fmt, "Neutral"),
            JoyDirection::Invalid => defmt::write!(fmt, "Invalid"),
        }
    }
}

/// Analog joystick via ADC resistor-ladder on a single pin.
pub struct Joystick<P: AdcChannel<ADC1>> {
    adc: Adc<'static, ADC1>,
    pin: P,
}

impl<P: AdcChannel<ADC1>> Joystick<P> {
    pub fn new(adc: Peri<'static, ADC1>, pin: P) -> Self {
        Self {
            adc: Adc::new(adc),
            pin,
        }
    }

    pub fn read(&mut self) -> JoyDirection {
        let value = self.adc.blocking_read(&mut self.pin, SampleTime::CYCLES7_5);
        match value {
            v if (CENTER_THRESHOLD_MIN..=CENTER_THRESHOLD_MAX).contains(&v) => JoyDirection::Center,
            v if (LEFT_THRESHOLD_MIN..=LEFT_THRESHOLD_MAX).contains(&v) => JoyDirection::Left,
            v if (DOWN_THRESHOLD_MIN..=DOWN_THRESHOLD_MAX).contains(&v) => JoyDirection::Down,
            v if (UP_THRESHOLD_MIN..=UP_THRESHOLD_MAX).contains(&v) => JoyDirection::Up,
            v if (RIGHT_THRESHOLD_MIN..=RIGHT_THRESHOLD_MAX).contains(&v) => JoyDirection::Right,
            v if (NEUTRAL_THRESHOLD_MIN..=NEUTRAL_THRESHOLD_MAX).contains(&v) => {
                JoyDirection::Neutral
            }
            _ => JoyDirection::Invalid,
        }
    }
}

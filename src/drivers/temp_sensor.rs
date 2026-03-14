use core::fmt::Display;

use embassy_stm32::{
    Peri,
    i2c::{self, I2c, Master},
    mode::Blocking,
    peripherals::{I2C1, PB7, PB8},
    time::Hertz,
};

const ADDR: u8 = 0x3F;

const REG_WHOAMI: u8 = 0x01;
const REG_CTRL: u8 = 0x04;
const REG_STATUS: u8 = 0x05;
const REG_TEMP_L_OUT: u8 = 0x06;
const REG_SOFTWARE_RESET: u8 = 0x0C;

const WHOAMI_VALUE: u8 = 0xA0;
const CTRL_IF_ADD_INC: u8 = 0x08;
const CTRL_ONE_SHOT: u8 = 0x01;
const STATUS_BUSY: u8 = 0x01;

pub struct Stts22h {
    i2c: I2c<'static, Blocking, Master>,
}

#[derive(Debug)]
pub enum Stts22hError {
    I2c(i2c::Error),
    WhoAmI(u8),
    Timeout,
}

impl defmt::Format for Stts22hError {
    #[allow(clippy::match_same_arms)]
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            Self::I2c(err) => defmt::write!(fmt, "I2C communication failed: {}", err),
            Self::WhoAmI(id) => defmt::write!(fmt, "WhoAmI failed: {}", id),
            Self::Timeout => defmt::write!(fmt, "Timeout"),
        }
    }
}

impl Display for Stts22hError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::I2c(err) => write!(f, "I2C communication failed: {err}"),
            Self::WhoAmI(id) => write!(f, "WhoAmI failed: {id}"),
            Self::Timeout => write!(f, "Timeout"),
        }
    }
}

impl Stts22h {
    /// Create a new STTS22H sensor.
    ///
    /// # Errors
    ///
    /// Returns an error if the I2C communication fails.
    pub fn new(
        peri: Peri<'static, I2C1>,
        scl: Peri<'static, PB8>,
        sda: Peri<'static, PB7>,
    ) -> Result<Self, Stts22hError> {
        let mut config = i2c::Config::default();
        config.frequency = Hertz(100_000);
        let i2c = I2c::new_blocking(peri, scl, sda, config);
        let mut sensor = Self { i2c };
        sensor.verify_whoami()?;
        Ok(sensor)
    }

    fn verify_whoami(&mut self) -> Result<(), Stts22hError> {
        let id = self.read_reg(REG_WHOAMI)?;
        if id != WHOAMI_VALUE {
            return Err(Stts22hError::WhoAmI(id));
        }
        Ok(())
    }

    /// Trigger a one-shot measurement and return temperature in degrees Celsius.
    ///
    /// # Errors
    ///
    /// Returns an error if the measurement fails.
    pub fn read_temperature(&mut self) -> Result<f32, Stts22hError> {
        self.write_reg(REG_SOFTWARE_RESET, 0x02)?;
        self.write_reg(REG_SOFTWARE_RESET, 0x00)?;
        self.write_reg(REG_CTRL, CTRL_IF_ADD_INC | CTRL_ONE_SHOT)?;

        let mut retries = 200u16;
        loop {
            let status = self.read_reg(REG_STATUS)?;
            if status & STATUS_BUSY == 0 {
                break;
            }
            retries = retries.checked_sub(1).ok_or(Stts22hError::Timeout)?;
        }

        let mut buf = [0u8; 2];
        self.i2c
            .blocking_write_read(ADDR, &[REG_TEMP_L_OUT], &mut buf)
            .map_err(Stts22hError::I2c)?;

        let raw = i16::from_le_bytes(buf);
        Ok(f32::from(raw) / 100.0)
    }

    fn read_reg(&mut self, reg: u8) -> Result<u8, Stts22hError> {
        let mut buf = [0u8; 1];
        self.i2c
            .blocking_write_read(ADDR, &[reg], &mut buf)
            .map_err(Stts22hError::I2c)?;
        Ok(buf[0])
    }

    fn write_reg(&mut self, reg: u8, val: u8) -> Result<(), Stts22hError> {
        self.i2c
            .blocking_write(ADDR, &[reg, val])
            .map_err(Stts22hError::I2c)?;
        Ok(())
    }
}

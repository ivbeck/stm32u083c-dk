#![no_main]
#![no_std]

use defmt_rtt as _;
use panic_probe as _;

mod macros;
pub use macros::*;

pub mod communication;
pub mod drivers;
pub mod tasks;

use embassy_stm32::{
    Peri,
    lcd::{self, Bias, Config, Duty, LcdPin},
    peripherals::{
        self, LCD, PA8, PA9, PA10, PB1, PB9, PB11, PB14, PB15, PC3, PC4, PC5, PC6, PC8, PC9,
        PC10, PC11, PD0, PD1, PD3, PD4, PD5, PD6, PD8, PD9, PD12, PD13, PE7, PE8, PE9,
    },
};

// Glass SEG index (0..23 on DIP28 connector) -> hardware MCU LCD_SEGx bit position
// in the 64-bit value passed to `write_com_segments`.
// Source: UM3292 Table 12 (RevB/RevC board).
const GLASS_TO_HW_SEG: [u8; 24] = [
    22, // Glass SEG0  -> LCD_SEG22 (PC4)
    23, // Glass SEG1  -> LCD_SEG23 (PC5)
    6,  // Glass SEG2  -> LCD_SEG6  (PB1)
    45, // Glass SEG3  -> LCD_SEG45 (PE7)
    46, // Glass SEG4  -> LCD_SEG46 (PE8)
    47, // Glass SEG5  -> LCD_SEG47 (PE9)
    11, // Glass SEG6  -> LCD_SEG11 (PB11)
    14, // Glass SEG7  -> LCD_SEG14 (PB14)
    15, // Glass SEG8  -> LCD_SEG15 (PB15)
    28, // Glass SEG9  -> LCD_SEG28 (PD8)
    29, // Glass SEG10 -> LCD_SEG29 (PD9)
    32, // Glass SEG11 -> LCD_SEG32 (PD12)
    33, // Glass SEG12 -> LCD_SEG33 (PD13)
    24, // Glass SEG13 -> LCD_SEG24 (PC6)
    26, // Glass SEG14 -> LCD_SEG26 (PC8)
    27, // Glass SEG15 -> LCD_SEG27 (PC9)
    48, // Glass SEG16 -> LCD_SEG48 (PC10)
    34, // Glass SEG17 -> LCD_SEG34 (PD0)
    35, // Glass SEG18 -> LCD_SEG35 (PD1)
    36, // Glass SEG19 -> LCD_SEG36 (PD3)
    37, // Glass SEG20 -> LCD_SEG37 (PD4)
    38, // Glass SEG21 -> LCD_SEG38 (PD5)
    39, // Glass SEG22 -> LCD_SEG39 (PD6)
    49, // Glass SEG23 -> LCD_SEG49 (PC11)
];

// Per-digit glass SEG indices (4 per digit).
// Within each COM, the 4-bit nibble from the character encoding maps:
//   bit 0 -> glass_segs[0], bit 1 -> glass_segs[1],
//   bit 2 -> glass_segs[2], bit 3 -> glass_segs[3].
// Source: STM32CubeU0 BSP WriteChar (RevB/RevC).
const DIGIT_GLASS_SEGS: [[u8; 4]; 6] = [
    [0, 1, 22, 23],   // Digit 1 (leftmost)
    [2, 3, 20, 21],   // Digit 2
    [4, 5, 18, 19],   // Digit 3
    [6, 7, 16, 17],   // Digit 4
    [8, 9, 14, 15],   // Digit 5
    [10, 11, 12, 13], // Digit 6 (rightmost)
];

// 14-segment character encoding from STM32CubeU0 BSP.
// Packed as 16 bits: [COM0:4][COM1:4][COM2:4][COM3:4].
//
// Glass 14-segment layout:
//     -----A-----
//     |\   |   /|
//     F H  J  K B
//     |  \ | /  |
//     --G-- --M--
//     |  / | \  |
//     E Q  P  N C
//     |/   |   \|
//     -----D-----
//
// COM0 nibble bits: 0=E, 1=M, 2=B, 3=G
// COM1 nibble bits: 0=D, 1=C, 2=A, 3=F
// COM2 nibble bits: 0=P, 1=COL, 2=K, 3=Q
// COM3 nibble bits: 0=N, 1=DP,  2=J, 3=H
const NUMBER_MAP: [u16; 11] = [
    0x5F00, // 0
    0x4200, // 1
    0xF500, // 2
    0x6700, // 3
    0xEA00, // 4
    0xAF00, // 5
    0xBF00, // 6
    0x4600, // 7
    0xFF00, // 8
    0xEF00, // 9
    0x0000, // blank
];

/// Pin set for the STM32U083C-DK on-board 4x24 segment LCD (DIP28 connector).
/// Pin order follows UM3292 Table 12.
pub struct SegLcdPins {
    pub lcd: Peri<'static, LCD>,
    pub vlcd: Peri<'static, PC3>,
    pub com0: Peri<'static, PA8>,
    pub com1: Peri<'static, PA9>,
    pub com2: Peri<'static, PA10>,
    pub com3: Peri<'static, PB9>,
    pub seg0: Peri<'static, PC4>,
    pub seg1: Peri<'static, PC5>,
    pub seg2: Peri<'static, PB1>,
    pub seg3: Peri<'static, PE7>,
    pub seg4: Peri<'static, PE8>,
    pub seg5: Peri<'static, PE9>,
    pub seg6: Peri<'static, PB11>,
    pub seg7: Peri<'static, PB14>,
    pub seg8: Peri<'static, PB15>,
    pub seg9: Peri<'static, PD8>,
    pub seg10: Peri<'static, PD9>,
    pub seg11: Peri<'static, PD12>,
    pub seg12: Peri<'static, PD13>,
    pub seg13: Peri<'static, PC6>,
    pub seg14: Peri<'static, PC8>,
    pub seg15: Peri<'static, PC9>,
    pub seg16: Peri<'static, PC10>,
    pub seg17: Peri<'static, PD0>,
    pub seg18: Peri<'static, PD1>,
    pub seg19: Peri<'static, PD3>,
    pub seg20: Peri<'static, PD4>,
    pub seg21: Peri<'static, PD5>,
    pub seg22: Peri<'static, PD6>,
    pub seg23: Peri<'static, PC11>,
}

impl SegLcd {
    /// Convenience constructor for the STM32U083C-DK board.
    ///
    /// Steals the LCD-related peripheral singletons. After calling this, the
    /// corresponding fields on `Peripherals` (`LCD`, `PC3`, `PA8`, `PA9`,
    /// `PA10`, `PB1`, `PB9`, `PB11`, `PB14`, `PB15`, `PC4`..`PC11`,
    /// `PD0`, `PD1`, `PD3`..`PD6`, `PD8`, `PD9`, `PD12`, `PD13`,
    /// `PE7`..`PE9`) **must not be used again**.
    ///
    /// # Safety
    ///
    /// The caller must ensure these peripherals are not used elsewhere.
    /// In practice, call this once early in `main` before handing out any
    /// of the LCD-related pins.
    pub unsafe fn from_peripherals() -> Self {
        unsafe {
            Self::new(SegLcdPins {
                lcd: peripherals::LCD::steal(),
                vlcd: peripherals::PC3::steal(),
                com0: peripherals::PA8::steal(),
                com1: peripherals::PA9::steal(),
                com2: peripherals::PA10::steal(),
                com3: peripherals::PB9::steal(),
                seg0: peripherals::PC4::steal(),
                seg1: peripherals::PC5::steal(),
                seg2: peripherals::PB1::steal(),
                seg3: peripherals::PE7::steal(),
                seg4: peripherals::PE8::steal(),
                seg5: peripherals::PE9::steal(),
                seg6: peripherals::PB11::steal(),
                seg7: peripherals::PB14::steal(),
                seg8: peripherals::PB15::steal(),
                seg9: peripherals::PD8::steal(),
                seg10: peripherals::PD9::steal(),
                seg11: peripherals::PD12::steal(),
                seg12: peripherals::PD13::steal(),
                seg13: peripherals::PC6::steal(),
                seg14: peripherals::PC8::steal(),
                seg15: peripherals::PC9::steal(),
                seg16: peripherals::PC10::steal(),
                seg17: peripherals::PD0::steal(),
                seg18: peripherals::PD1::steal(),
                seg19: peripherals::PD3::steal(),
                seg20: peripherals::PD4::steal(),
                seg21: peripherals::PD5::steal(),
                seg22: peripherals::PD6::steal(),
                seg23: peripherals::PC11::steal(),
            })
        }
    }
}

pub struct SegLcd {
    lcd: lcd::Lcd<'static, LCD>,
}

impl SegLcd {
    pub fn new(pins: SegLcdPins) -> Self {
        let mut config = Config::default();
        config.duty = Duty::Quarter;
        config.bias = Bias::Third;

        let lcd = lcd::Lcd::new(
            pins.lcd,
            config,
            pins.vlcd,
            [
                LcdPin::new_com(pins.com0),
                LcdPin::new_com(pins.com1),
                LcdPin::new_com(pins.com2),
                LcdPin::new_com(pins.com3),
                LcdPin::new_seg(pins.seg0),
                LcdPin::new_seg(pins.seg1),
                LcdPin::new_seg(pins.seg2),
                LcdPin::new_seg(pins.seg3),
                LcdPin::new_seg(pins.seg4),
                LcdPin::new_seg(pins.seg5),
                LcdPin::new_seg(pins.seg6),
                LcdPin::new_seg(pins.seg7),
                LcdPin::new_seg(pins.seg8),
                LcdPin::new_seg(pins.seg9),
                LcdPin::new_seg(pins.seg10),
                LcdPin::new_seg(pins.seg11),
                LcdPin::new_seg(pins.seg12),
                LcdPin::new_seg(pins.seg13),
                LcdPin::new_seg(pins.seg14),
                LcdPin::new_seg(pins.seg15),
                LcdPin::new_seg(pins.seg16),
                LcdPin::new_seg(pins.seg17),
                LcdPin::new_seg(pins.seg18),
                LcdPin::new_seg(pins.seg19),
                LcdPin::new_seg(pins.seg20),
                LcdPin::new_seg(pins.seg21),
                LcdPin::new_seg(pins.seg22),
                LcdPin::new_seg(pins.seg23),
            ],
        );

        Self { lcd }
    }

    /// Display a decimal number right-justified across all 6 digit positions.
    /// Values above 999999 are clamped. Leading positions are blank.
    pub fn display_number(&mut self, value: u32) {
        let value = value.min(999999);

        let digits: [usize; 6] = [
            if value >= 100000 {
                (value / 100000 % 10) as usize
            } else {
                10
            },
            if value >= 10000 {
                (value / 10000 % 10) as usize
            } else {
                10
            },
            if value >= 1000 {
                (value / 1000 % 10) as usize
            } else {
                10
            },
            if value >= 100 {
                (value / 100 % 10) as usize
            } else {
                10
            },
            if value >= 10 {
                (value / 10 % 10) as usize
            } else {
                10
            },
            (value % 10) as usize,
        ];

        let mut com_segs = [0u64; 4];

        for (digit_idx, &digit_val) in digits.iter().enumerate() {
            let encoding = NUMBER_MAP[digit_val];
            let glass_segs = &DIGIT_GLASS_SEGS[digit_idx];

            for com in 0..4u8 {
                let nibble = (encoding >> (12 - com as u16 * 4)) & 0xf;
                for bit in 0..4u8 {
                    if (nibble & (1 << bit)) != 0 {
                        let glass_idx = glass_segs[bit as usize];
                        let hw_seg = GLASS_TO_HW_SEG[glass_idx as usize];
                        com_segs[com as usize] |= 1u64 << hw_seg;
                    }
                }
            }
        }

        for (com, &mask) in com_segs.iter().enumerate() {
            self.lcd.write_com_segments(com as u8, mask);
        }
        self.lcd.submit_frame();
    }

    /// Clear the entire LCD.
    pub fn clear(&mut self) {
        for c in 0..4u8 {
            self.lcd.write_com_segments(c, 0);
        }
        self.lcd.submit_frame();
    }

    /// Light a single glass segment for hardware bring-up / calibration.
    /// `glass_seg` is 0..23 (DIP28 connector pin order from UM3292 Table 12).
    /// `com` is 0..3.
    pub fn test_single_segment(&mut self, com: u8, glass_seg: u8) {
        let hw_seg = GLASS_TO_HW_SEG[glass_seg as usize];
        for c in 0..4u8 {
            self.lcd.write_com_segments(c, 0);
        }
        self.lcd.write_com_segments(com, 1u64 << hw_seg);
        self.lcd.submit_frame();
    }
}

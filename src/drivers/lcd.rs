use embassy_stm32::{
    lcd::{self, Bias, Config, Duty, LcdPin},
    peripherals::{
        LCD, PA8, PA9, PA10, PB1, PB9, PB11, PB14, PB15, PC3, PC4, PC5, PC6, PC8, PC9, PC10, PC11,
        PD0, PD1, PD3, PD4, PD5, PD6, PD8, PD9, PD12, PD13, PE7, PE8, PE9,
    },
    Peri,
};

// Standard 7-segment encoding.
// Bit positions: 0=a(top) 1=b(upper-right) 2=c(lower-right) 3=d(bottom)
//                4=e(lower-left) 5=f(upper-left) 6=g(middle)
//
//   _
//  |_|
//  |_|
const SEVEN_SEG: [u8; 11] = [
    0b0111111, // 0: abcdef
    0b0000110, // 1: bc
    0b1011011, // 2: abdeg
    0b1001111, // 3: abcdg
    0b1100110, // 4: bcfg
    0b1101101, // 5: acdfg
    0b1111101, // 6: acdefg
    0b0000111, // 7: abc
    0b1111111, // 8: abcdefg
    0b1101111, // 9: abcdfg
    0b0000000, // 10: blank
];

/// Mapping from logical digit & segment (a-g) to `(com_index, seg_index)` on the LCD controller.
///
/// - First index: digit position 0..=3 (left to right)
/// - Second index: segment 0..=6 (a..g as in `SEVEN_SEG`)
/// - Value: `(com, seg)` such that we light `COM[com]` + `SEG[seg]`.
///
/// IMPORTANT: The values below are **placeholders**. They must be updated to match
/// the actual glass mapping you discover with `test_segments` (see `main.rs`).
const DIGIT_SEG_MAP: [[(u8, u8); 7]; 4] = [
    // Digit 0
    [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (0, 2), (1, 2)],
    // Digit 1
    [(0, 3), (0, 4), (1, 3), (1, 4), (2, 1), (0, 5), (1, 5)],
    // Digit 2
    [(0, 6), (0, 7), (1, 6), (1, 7), (2, 2), (0, 8), (1, 8)],
    // Digit 3
    [(0, 9), (0, 10), (1, 9), (1, 10), (2, 3), (0, 11), (1, 11)],
];

// Physical segment mapping: DIGIT_MAP[digit_position][segment_a_through_g] = (com_index, seg_bit)
//
// ══════════════════════════════════════════════════════════════════════════
// CALIBRATION REQUIRED — values below are PLACEHOLDER guesses based on the
// typical STM32 glass LCD pinout (2 SEG lines per digit, 4 COMs).
//
// How to calibrate:
//   1. Call lcd.write_com_segments(com, 1u64 << seg_n) for each COM (0-3)
//      and each segment number from the board's pin table.
//   2. Note which physical pixel lights up on the glass.
//   3. Fill in this table to map each visual segment (a-g) of each digit
//      to the correct (com, 1u64 << seg_n) pair.
//
// Assumed glass layout (2 SEG lines per digit, pins from DIP28 connector):
//   Digit 0: HW SEG22 (glass left), HW SEG23 (glass right)
//   Digit 1: HW SEG6  (glass left), HW SEG45 (glass right)
//   Digit 2: HW SEG46 (glass left), HW SEG47 (glass right)
//   Digit 3: HW SEG11 (glass left), HW SEG14 (glass right)
//
// Assumed COM-to-segment mapping per digit (common STM32 glass convention):
//   COM0 + left  → f (upper-left)   COM0 + right → a (top)
//   COM1 + left  → g (middle)       COM1 + right → b (upper-right)
//   COM2 + left  → e (lower-left)   COM2 + right → c (lower-right)
//   COM3 + left  → d (bottom)       COM3 + right → dp (decimal point)
// ══════════════════════════════════════════════════════════════════════════
//
//                   a             b             c             d
//                   e             f             g
pub struct SegLcd {
    lcd: lcd::Lcd<'static, LCD>,
}

impl SegLcd {
    pub fn new(
        lcd_peripheral: Peri<'static, LCD>,
        vlcd_pin: Peri<'static, PC3>,
        com0: Peri<'static, PA8>,
        com1: Peri<'static, PA9>,
        com2: Peri<'static, PA10>,
        com3: Peri<'static, PB9>,
        seg0: Peri<'static, PC4>,
        seg1: Peri<'static, PC5>,
        seg2: Peri<'static, PB1>,
        seg3: Peri<'static, PE7>,
        seg4: Peri<'static, PE8>,
        seg5: Peri<'static, PE9>,
        seg6: Peri<'static, PB11>,
        seg7: Peri<'static, PB14>,
        seg8: Peri<'static, PB15>,
        seg9: Peri<'static, PD8>,
        seg10: Peri<'static, PD9>,
        seg11: Peri<'static, PD12>,
        seg12: Peri<'static, PD13>,
        seg13: Peri<'static, PC6>,
        seg14: Peri<'static, PC8>,
        seg15: Peri<'static, PC9>,
        seg16: Peri<'static, PC10>,
        seg17: Peri<'static, PD0>,
        seg18: Peri<'static, PD1>,
        seg19: Peri<'static, PD3>,
        seg20: Peri<'static, PD4>,
        seg21: Peri<'static, PD5>,
        seg22: Peri<'static, PD6>,
        seg23: Peri<'static, PC11>,
    ) -> Self {
        let mut config = Config::default();
        config.duty = Duty::Quarter;
        config.bias = Bias::Third;

        let lcd = lcd::Lcd::new(
            lcd_peripheral,
            config,
            vlcd_pin,
            [
                LcdPin::new_com(com0),
                LcdPin::new_com(com1),
                LcdPin::new_com(com2),
                LcdPin::new_com(com3),
                LcdPin::new_seg(seg0),
                LcdPin::new_seg(seg1),
                LcdPin::new_seg(seg2),
                LcdPin::new_seg(seg3),
                LcdPin::new_seg(seg4),
                LcdPin::new_seg(seg5),
                LcdPin::new_seg(seg6),
                LcdPin::new_seg(seg7),
                LcdPin::new_seg(seg8),
                LcdPin::new_seg(seg9),
                LcdPin::new_seg(seg10),
                LcdPin::new_seg(seg11),
                LcdPin::new_seg(seg12),
                LcdPin::new_seg(seg13),
                LcdPin::new_seg(seg14),
                LcdPin::new_seg(seg15),
                LcdPin::new_seg(seg16),
                LcdPin::new_seg(seg17),
                LcdPin::new_seg(seg18),
                LcdPin::new_seg(seg19),
                LcdPin::new_seg(seg20),
                LcdPin::new_seg(seg21),
                LcdPin::new_seg(seg22),
                LcdPin::new_seg(seg23),
            ],
        );

        Self { lcd }
    }

    /// Display a decimal value right-justified across 4 digits (leading blanks).
    /// Values above 9999 are clamped.
    pub fn display_ms(&mut self, ms: u32) {
        let ms = ms.min(9999);

        let digits = [
            if ms >= 1000 {
                (ms / 1000 % 10) as usize
            } else {
                10
            },
            if ms >= 100 {
                (ms / 100 % 10) as usize
            } else {
                10
            },
            if ms >= 10 {
                (ms / 10 % 10) as usize
            } else {
                10
            },
            (ms % 10) as usize,
        ];

        // Accumulate a 64-bit segment bitmap per COM line.
        let mut com_segs = [0u64; 4];

        for (digit_idx, &digit) in digits.iter().enumerate() {
            if digit == 10 {
                // Blank
                continue;
            }

            let pattern = SEVEN_SEG[digit];

            for seg_idx in 0..7 {
                if (pattern & (1 << seg_idx)) == 0 {
                    continue;
                }

                let (com, seg) = DIGIT_SEG_MAP[digit_idx][seg_idx];

                if com >= 4 {
                    continue;
                }
                if seg as u32 >= 64 {
                    continue;
                }

                com_segs[com as usize] |= 1u64 << seg;
            }
        }

        // Push frame to hardware.
        for (com, &mask) in com_segs.iter().enumerate() {
            self.lcd.write_com_segments(com as u8, mask);
        }
        self.lcd.submit_frame();
    }

    pub fn test_single_segment(&mut self, com: u8, seg_bit: u64) {
        // Clear all segments.
        for c in 0..4u8 {
            self.lcd.write_com_segments(c, 0);
        }
        // Light exactly one COM/SEG combination.
        self.lcd.write_com_segments(com, seg_bit);
        self.lcd.submit_frame();
    }
}

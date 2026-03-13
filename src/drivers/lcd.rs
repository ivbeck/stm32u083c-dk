use embassy_futures::select::{Either, select};
use embassy_stm32::{
    Peri,
    lcd::{self, Bias, Config, Duty, LcdPin},
    peripherals::{
        self, LCD, PA8, PA9, PA10, PB1, PB9, PB11, PB14, PB15, PC3, PC4, PC5, PC6, PC8, PC9, PC10,
        PC11, PD0, PD1, PD3, PD4, PD5, PD6, PD8, PD9, PD12, PD13, PE7, PE8, PE9,
    },
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Timer;

pub type LcdChannel = Channel<CriticalSectionRawMutex, LcdMessage, 4>;

const LCD_TEXT_MAX: usize = 32;

#[derive(Clone)]
pub enum LcdMessage {
    Text {
        buf: [u8; LCD_TEXT_MAX],
        len: u8,
        speed_ms: u16,
    },
    Clear,
}

impl LcdMessage {
    #[must_use]
    pub fn text(s: &str, speed_ms: u16) -> Self {
        let mut buf = [b' '; LCD_TEXT_MAX];
        let len = s.len().min(LCD_TEXT_MAX);
        buf[..len].copy_from_slice(&s.as_bytes()[..len]);
        Self::Text {
            buf,
            #[allow(clippy::cast_possible_truncation)]
            len: len as u8,
            speed_ms,
        }
    }
}

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
const DIGIT_MAP: [u16; 10] = [
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
];

const LETTER_MAP: [u16; 26] = [
    0xFE00, // A
    0x6714, // B
    0x1D00, // C
    0x4714, // D
    0x9D00, // E
    0x9C00, // F
    0x3F00, // G
    0xFA00, // H
    0x0014, // I
    0x5300, // J
    0x9841, // K
    0x1900, // L
    0x5A48, // M
    0x5A09, // N
    0x5F00, // O
    0xFC00, // P
    0x5F01, // Q
    0xFC01, // R
    0xAF00, // S
    0x0414, // T
    0x5B00, // U
    0x18C0, // V
    0x5A81, // W
    0x00C9, // X
    0x0058, // Y
    0x05C0, // Z
];

const BLANK: u16 = 0x0000;

const fn char_encoding(ch: u8) -> u16 {
    match ch {
        b'0'..=b'9' => DIGIT_MAP[(ch - b'0') as usize],
        b'A'..=b'Z' => LETTER_MAP[(ch - b'A') as usize],
        b'a'..=b'z' => LETTER_MAP[(ch - b'a') as usize],
        b'-' => 0xA000,
        b'+' => 0xA014,
        b'*' => 0xA0DD,
        b'/' => 0x00C0,
        b'(' => 0x0028,
        b')' => 0x0011,
        b'%' => 0xB300,
        b'_' => 0x0100,
        b',' | b'.' => 1 << 12,
        _ => BLANK,
    }
}

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
    #[must_use]
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
    last_frame: [u16; 6],
}

impl SegLcd {
    #[must_use]
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

        Self {
            lcd,
            last_frame: [BLANK; 6],
        }
    }

    /// Write 6 character encodings to the LCD hardware and submit the frame.
    fn write_frame(&mut self, encodings: &[u16; 6]) {
        self.last_frame = *encodings;
        let mut com_segs = [0u64; 4];

        for (digit_idx, &encoding) in encodings.iter().enumerate() {
            let glass_segs = &DIGIT_GLASS_SEGS[digit_idx];

            for com in 0..4u8 {
                let nibble = (encoding >> (12 - u16::from(com) * 4)) & 0xf;
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
            #[allow(clippy::cast_possible_truncation)]
            self.lcd.write_com_segments(com as u8, mask);
        }
        self.lcd.submit_frame();
    }

    /// Display a decimal number right-justified across all 6 digit positions.
    /// Values above 999999 are clamped. Leading positions are blank.
    pub fn display_number(&mut self, value: u32) {
        let value = value.min(999_999);
        let divs = [100_000, 10000, 1000, 100, 10, 1];
        let mut encodings = [BLANK; 6];

        for (i, &d) in divs.iter().enumerate() {
            if value >= d || d == 1 {
                encodings[i] = DIGIT_MAP[((value / d) % 10) as usize];
            }
        }

        self.write_frame(&encodings);
    }

    /// Display a string left-justified across the 6 digit positions.
    /// Only the first 6 characters are shown; remaining positions are blank.
    /// Supports A-Z, a-z, 0-9, space, and common punctuation (-+*/%()).
    pub fn display_str(&mut self, s: &str) {
        let mut encodings = [BLANK; 6];

        for (i, &byte) in s.as_bytes().iter().take(6).enumerate() {
            encodings[i] = char_encoding(byte);
        }

        self.write_frame(&encodings);
    }

    /// Clear the entire LCD.
    pub fn clear(&mut self) {
        self.last_frame = [BLANK; 6];
        for c in 0..4u8 {
            self.lcd.write_com_segments(c, 0);
        }
        self.lcd.submit_frame();
    }

    /// Light a single glass segment for hardware bring-up / calibration.
    /// `glass_seg` is 0..23 (DIP28 connector pin order from UM3292 Table 12).
    /// `com` is 0..3.
    #[allow(unused)]
    pub fn test_single_segment(&mut self, com: u8, glass_seg: u8) {
        let hw_seg = GLASS_TO_HW_SEG[glass_seg as usize];
        for c in 0..4u8 {
            self.lcd.write_com_segments(c, 0);
        }
        self.lcd.write_com_segments(com, 1u64 << hw_seg);
        self.lcd.submit_frame();
    }

    /// Process LCD messages from a queue, forever.
    ///
    /// Always loops the latest text entry. When a new message arrives,
    /// a dash-wipe transition plays before switching to the new text.
    ///
    /// Spawn this in a dedicated `#[embassy_executor::task]`.
    pub async fn run(&mut self, channel: &'static LcdChannel) -> ! {
        let mut msg = channel.receive().await;
        let mut first = true;

        loop {
            msg = Self::drain_channel(channel, msg);

            match msg {
                LcdMessage::Clear => {
                    self.clear();
                    msg = channel.receive().await;
                    first = true;
                }
                LcdMessage::Text { buf, len, speed_ms } => {
                    let text = &buf[..len as usize];

                    if !first {
                        self.play_transition(text).await;
                        if let Ok(newer) = channel.try_receive() {
                            msg = Self::drain_channel(channel, newer);
                            continue;
                        }
                    }
                    first = false;

                    msg = self.scroll_loop(channel, text, u64::from(speed_ms)).await;
                }
            }
        }
    }

    /// Drain all pending messages from the channel, returning the latest.
    fn drain_channel(channel: &LcdChannel, initial: LcdMessage) -> LcdMessage {
        let mut latest = initial;
        while let Ok(msg) = channel.try_receive() {
            latest = msg;
        }
        latest
    }

    /// Scroll text in an infinite loop, returning when a new message arrives.
    ///
    /// Short text (<=6 chars) is displayed statically until interrupted.
    async fn scroll_loop(
        &mut self,
        channel: &LcdChannel,
        text: &[u8],
        speed_ms: u64,
    ) -> LcdMessage {
        const GAP: usize = 3;

        if text.len() <= 6 {
            let mut encodings = [BLANK; 6];
            for (i, &byte) in text.iter().enumerate() {
                encodings[i] = char_encoding(byte);
            }
            self.write_frame(&encodings);
            return channel.receive().await;
        }

        let period = text.len() + GAP;
        loop {
            for offset in 0..period {
                let mut encodings = [BLANK; 6];
                for (i, enc) in encodings.iter_mut().enumerate() {
                    let pos = (offset + i) % period;
                    if pos < text.len() {
                        *enc = char_encoding(text[pos]);
                    }
                }
                self.write_frame(&encodings);

                match select(Timer::after_millis(speed_ms), channel.receive()).await {
                    Either::First(()) => {}
                    Either::Second(new_msg) => return new_msg,
                }
            }
        }
    }

    /// Dash-wipe transition: dashes sweep left-to-right over the current
    /// content, then the new text is revealed left-to-right.
    async fn play_transition(&mut self, new_text: &[u8]) {
        const STEP_MS: u64 = 40;
        let dash = char_encoding(b'-');

        let mut new_frame = [BLANK; 6];
        for (i, enc) in new_frame.iter_mut().enumerate() {
            if i < new_text.len() {
                *enc = char_encoding(new_text[i]);
            }
        }

        let mut frame = self.last_frame;

        for i in 0..6 {
            frame[i] = dash;
            self.write_frame(&frame);
            Timer::after_millis(STEP_MS).await;
        }

        for i in 0..6 {
            frame[i] = new_frame[i];
            self.write_frame(&frame);
            Timer::after_millis(STEP_MS).await;
        }
    }
}

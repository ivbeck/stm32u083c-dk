use crate::{communication::LCD_CMD, drivers::lcd::SegLcd};

#[embassy_executor::task]
pub async fn lcd_task(mut lcd: SegLcd) {
    lcd.run(&LCD_CMD).await;
}

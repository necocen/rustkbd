use atmega_hal::{
    clock::MHz16,
    pac::TWI,
    port::{
        mode::{Input, PullUp},
        Pin, PD0, PD1,
    },
    I2c,
};
use embedded_graphics::{pixelcolor::BinaryColor, Drawable};
use keyboard_core::oled::OledModule;
use ssd1306::{
    mode::{BufferedGraphicsMode, DisplayConfig},
    prelude::I2CInterface,
    rotation::DisplayRotation,
    size::DisplaySize128x32,
    I2CDisplayInterface, Ssd1306 as Driver,
};

pub struct Ssd1306 {
    display: Driver<
        I2CInterface<I2c<MHz16>>,
        DisplaySize128x32,
        BufferedGraphicsMode<DisplaySize128x32>,
    >,
}

impl Ssd1306 {
    pub fn new(twi: TWI, sda: Pin<Input<PullUp>, PD1>, scl: Pin<Input<PullUp>, PD0>) -> Self {
        let i2c = I2c::<MHz16>::new(twi, sda, scl, 51200);
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Driver::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().ok();
        Ssd1306 { display }
    }
}

impl OledModule for Ssd1306 {
    fn draw_char(&mut self, c: char) {
        todo!()
    }

    fn set_cursor(&mut self, x: usize, y: usize) {
        todo!()
    }

    fn clear(&mut self) {
        self.display.clear();
        self.display.flush().ok();
    }

    fn draw_image(&mut self, image: impl Drawable<Color = BinaryColor>) {
        image.draw(&mut self.display).ok();
        self.display.flush().ok();
    }
}

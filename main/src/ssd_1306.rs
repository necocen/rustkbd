use atmega_hal::{
    clock::MHz16,
    pac::TWI,
    port::{
        mode::{Input, PullUp},
        Pin, PD0, PD1,
    },
    I2c,
};
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::{ImageDrawableExt, Point, Size},
    primitives::Rectangle,
    Drawable,
};
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
    cursor_x: usize,
    cursor_y: usize,
}

impl Ssd1306 {
    const FONT_6X10: ImageRaw<'static, BinaryColor> =
        ImageRaw::new_binary(include_bytes!("./font_5x8.raw"), 80); // borrowed from embedded-graphics crate
    pub fn new(twi: TWI, sda: Pin<Input<PullUp>, PD1>, scl: Pin<Input<PullUp>, PD0>) -> Self {
        let i2c = I2c::<MHz16>::new(twi, sda, scl, 51200);
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Driver::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().ok();

        Ssd1306 {
            display,
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}

impl OledModule for Ssd1306 {
    fn draw_char(&mut self, c: char) {
        let p = ((c as u8) - (' ' as u8)) as i32;
        let row = p & 0x0f;
        let col = p >> 4;
        let glyph = Self::FONT_6X10.sub_image(&Rectangle::new(
            Point::new(row * 5, col * 8),
            Size::new(5, 8),
        ));
        let im = Image::new(
            &glyph,
            Point::new((self.cursor_x as i32) * 5, (self.cursor_y as i32) * 8),
        );
        //im.draw(&mut self.display).ok();
        self.draw(im);
        self.cursor_y += (self.cursor_x + 1) >> 4;
        self.cursor_x = (self.cursor_x + 1) & 0x0f;
    }

    fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    fn clear(&mut self) {
        self.display.clear();
    }

    fn draw(&mut self, drawable: impl Drawable<Color = BinaryColor>) {
        drawable.draw(&mut self.display).ok();
    }

    fn flush(&mut self) {
        self.display.flush().ok();
    }
}

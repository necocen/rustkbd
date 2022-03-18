use embedded_graphics::{draw_target::DrawTarget, prelude::*, primitives::Rectangle};
use rustkbd_core::display::KeyboardDisplay;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, size::DisplaySize, Ssd1306};

pub struct Ssd1306Display<DI: WriteOnlyDataCommand, SIZE: DisplaySize>(
    pub Ssd1306<DI, SIZE, BufferedGraphicsMode<SIZE>>,
);

impl<DI: WriteOnlyDataCommand, SIZE: DisplaySize> KeyboardDisplay for Ssd1306Display<DI, SIZE> {
    const REQUIRES_FLUSH: bool = true;

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<DI: WriteOnlyDataCommand, SIZE: DisplaySize> Dimensions for Ssd1306Display<DI, SIZE> {
    fn bounding_box(&self) -> Rectangle {
        self.0.bounding_box()
    }
}

impl<DI: WriteOnlyDataCommand, SIZE: DisplaySize> DrawTarget for Ssd1306Display<DI, SIZE> {
    type Color = <Ssd1306<DI, SIZE, BufferedGraphicsMode<SIZE>> as DrawTarget>::Color;
    type Error = <Ssd1306<DI, SIZE, BufferedGraphicsMode<SIZE>> as DrawTarget>::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        self.0.draw_iter(pixels)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        self.0.fill_contiguous(area, colors)
    }
}

use embedded_graphics::{pixelcolor::BinaryColor, Drawable};

pub trait OledModule {
    fn draw_char(&mut self, c: char);

    fn set_cursor(&mut self, x: usize, y: usize);

    fn clear(&mut self);

    fn draw_image(&mut self, image: impl Drawable<Color = BinaryColor>);
}

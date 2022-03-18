use embedded_graphics::draw_target::DrawTarget;

pub trait KeyboardDisplay: DrawTarget {
    const REQUIRES_FLUSH: bool;
    fn flush(&mut self) -> Result<(), Self::Error>;
}

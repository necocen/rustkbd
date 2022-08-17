use embedded_graphics::{
    mono_font::{ascii::FONT_9X15, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point},
    text::Text,
    Drawable,
};
use heapless::String;
use rustkbd_core::keyboard::KeyboardState;

use crate::layout::Layer;

pub fn draw_state(
    display: &mut impl DrawTarget<Color = BinaryColor>,
    state: KeyboardState<Layer, 6>,
) {
    let char_style = MonoTextStyle::new(&FONT_9X15, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

    Text::new("necoboard v1", Point::new(0, 15), char_style)
        .draw(display)
        .ok();

    // print pressed keys
    let mut string = String::<6>::new();
    state
        .keys
        .into_iter()
        .filter(|key| key.is_keyboard_key())
        .map(From::from)
        .for_each(|c| {
            string.push(c).ok();
        });
    Text::new(string.as_str(), Point::new(0, 32), char_style)
        .draw(display)
        .ok();

    // display Layer
    let layer = match state.layer {
        Layer::Default => "Default",
        Layer::Lower => "Lower",
        Layer::Raise => "Raise",
    };
    Text::new(layer, Point::new(0, 49), char_style)
        .draw(display)
        .ok();
}

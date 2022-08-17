use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point},
    text::Text,
    Drawable,
};
use heapless::String;
use rustkbd_core::{keyboard::KeyboardState, split::SplitState};

use crate::layout::Layer;

pub fn draw_state(
    display: &mut impl DrawTarget<Color = BinaryColor>,
    state: KeyboardState<Layer, 6>,
) {
    let char_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    display.clear(BinaryColor::Off).ok();

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
    Text::new(string.as_str(), Point::new(0, 10), char_style)
        .draw(display)
        .ok();

    // display "Receiver" or "Controller"
    let split = match state.split {
        SplitState::Undetermined => "Undetermined",
        SplitState::NotAvailable => "N/A",
        SplitState::Controller => "Controller",
        SplitState::Receiver => "Receiver",
    };
    Text::new(split, Point::new(0, 22), char_style)
        .draw(display)
        .ok();

    // display Layer
    let layer = match state.layer {
        Layer::Default => "Default",
        Layer::Lower => "Lower",
        Layer::Raise => "Raise",
    };
    Text::new(layer, Point::new(0, 34), char_style)
        .draw(display)
        .ok();
}

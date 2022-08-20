use embedded_graphics::{
    mono_font::{ascii::FONT_9X15, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::{DrawTarget, Point},
    text::Text,
    Drawable,
};
use embedded_hal::spi::MODE_0;
use embedded_time::rate::{Extensions, Hertz};
use heapless::String;
use rp2040_hal::{
    gpio::{
        bank0::{Gpio10, Gpio11, Gpio8, Gpio9},
        FunctionSpi, Pin, PushPullOutput,
    },
    spi::Enabled,
    Spi,
};
use rp_pico::pac::{self, SPI1};
use rustkbd_core::keyboard::KeyboardState;
use ssd1306::{
    mode::BufferedGraphicsMode,
    prelude::{DisplayConfig, SPIInterface},
    rotation::DisplayRotation,
    size::DisplaySize128x64,
    Ssd1306,
};

use crate::layout::Layer;

pub fn draw_state<const RO: usize>(
    display: &mut impl DrawTarget<Color = BinaryColor>,
    state: KeyboardState<Layer, RO>,
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

#[allow(clippy::type_complexity)]
pub fn display(
    spi1: SPI1,
    resets: &mut pac::RESETS,
    freq: Hertz,
    dc: Pin<Gpio8, PushPullOutput>,
    cs: Pin<Gpio9, PushPullOutput>,
    _gpio10: Pin<Gpio10, FunctionSpi>,
    _gpio11: Pin<Gpio11, FunctionSpi>,
) -> Ssd1306<
    SPIInterface<Spi<Enabled, SPI1, 8>, Pin<Gpio8, PushPullOutput>, Pin<Gpio9, PushPullOutput>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
> {
    let spi = Spi::<_, _, 8>::new(spi1).init(resets, freq, 16_000_000u32.Hz(), &MODE_0);
    let interface = SPIInterface::new(spi, dc, cs);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().ok();
    display
}

use core::cell::RefCell;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::Text,
    Drawable,
};
use heapless::{String, Vec};

use crate::{display::KeyboardDisplay, key_switches::KeySwitches};

pub struct Keyboard<K: KeySwitches<Identifier = (u8, u8)>, D: KeyboardDisplay<Color = BinaryColor>>
{
    key_switches: K,
    display: RefCell<D>,
    char_style: MonoTextStyle<'static, BinaryColor>,
}

impl<K: KeySwitches<Identifier = (u8, u8)>, D: KeyboardDisplay<Color = BinaryColor>>
    Keyboard<K, D>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];

    pub fn new(key_switches: K, display: D) -> Self {
        Keyboard {
            key_switches,
            display: RefCell::new(display),
            char_style: MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
        }
    }

    pub fn main_loop(&self) -> [u8; 6] {
        //let im = Image::new(&Self::RUST_LOGO, Point::new(0, 0));
        let left = self.key_switches.scan();
        //self.usart_controller.put(&left);
        //let right = self.usart_controller.get();
        let mut display = self.display.borrow_mut();
        display.clear(BinaryColor::Off).ok();
        //im.draw(&mut *display).ok();
        let keys = self.key_codes(left, None);
        let mut string = String::<6>::new();
        for key in keys.iter() {
            if *key != 0 {
                string.push((key - 0x1e + b'1') as char).ok();
            }
        }
        Text::new(string.as_str(), Point::new(0, 10), self.char_style)
            .draw(&mut *display)
            .ok();

        if D::REQUIRES_FLUSH {
            display.flush().ok();
        }
        keys
    }

    fn key_codes(&self, left: Vec<(u8, u8), 6>, right: Option<Vec<(u8, u8), 6>>) -> [u8; 6] {
        let mut keys = [0u8; 6];
        let right = right
            .map(|r| r.into_iter())
            .unwrap_or_else(|| Vec::new().into_iter());
        for (i, (col, row)) in left.into_iter().chain(right).enumerate() {
            keys[i] = Self::KEY_CODES_LEFT[col as usize][row as usize];
        }
        keys
    }
}

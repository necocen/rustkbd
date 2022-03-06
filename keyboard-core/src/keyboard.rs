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
    is_recv: bool,
}

impl<K: KeySwitches<Identifier = (u8, u8)>, D: KeyboardDisplay<Color = BinaryColor>>
    Keyboard<K, D>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];

    pub fn new(key_switches: K, display: D, is_recv: bool) -> Self {
        Keyboard {
            key_switches,
            display: RefCell::new(display),
            is_recv,
        }
    }

    pub fn main_loop(&self) -> [u8; 6] {
        //let im = Image::new(&Self::RUST_LOGO, Point::new(0, 0));
        //self.usart_controller.put(&left);
        //let right = self.usart_controller.get();
        //im.draw(&mut *display).ok();

        // scan key matrix
        let scan = self.key_switches.scan();
        let keys = if self.is_recv {
            self.key_codes(Vec::new(), scan)
        } else {
            self.key_codes(scan, Vec::new())
        };

        // setup display
        let mut display = self.display.borrow_mut();
        display.clear(BinaryColor::Off).ok();

        // print pressed keys
        let mut string = String::<6>::new();
        for key in keys.iter() {
            if *key != 0 {
                string.push((key - 0x1e + b'1') as char).ok();
            }
        }
        let char_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        Text::new(string.as_str(), Point::new(0, 10), char_style)
            .draw(&mut *display)
            .ok();

        // display "Receiver" or "Controller"
        if self.is_recv {
            Text::new("Receiver", Point::new(0, 22), char_style)
                .draw(&mut *display)
                .ok();
        } else {
            Text::new("Controller", Point::new(0, 22), char_style)
                .draw(&mut *display)
                .ok();
        }

        if D::REQUIRES_FLUSH {
            display.flush().ok();
        }
        keys
    }

    fn key_codes(&self, left: Vec<(u8, u8), 6>, right: Vec<(u8, u8), 6>) -> [u8; 6] {
        let mut keys = [0u8; 6];

        let left = left
            .into_iter()
            .map(|(col, row)| Self::KEY_CODES_LEFT[col as usize][row as usize]);
        let right = right
            .into_iter()
            .map(|(col, row)| Self::KEY_CODES_RIGHT[col as usize][row as usize]);

        for (i, key) in left.chain(right).take(6).enumerate() {
            keys[i] = key;
        }
        keys
    }
}

use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::Point,
};
use heapless::Vec;

use crate::key_switches::KeySwitches;

pub struct Keyboard<K: KeySwitches> {
    key_switches: K,
}

impl<K: KeySwitches<Identifier = (u8, u8)>> Keyboard<K> {
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];
    const RUST_LOGO: ImageRaw<'static, BinaryColor> =
        ImageRaw::new_binary(include_bytes!("./rust.raw"), 64);
    pub fn new(key_switches: K) -> Self {
        Keyboard { key_switches }
    }

    pub fn main_loop(&self) -> [u8; 6] {
        //let im = Image::new(&Self::RUST_LOGO, Point::new(0, 0));
        let left = self.key_switches.scan();
        //self.usart_controller.put(&left);
        //let right = self.usart_controller.get();
        self.key_codes(left, None)
        // self.oled_module.set_cursor(0, 0);
        // self.oled_module.clear();
        //self.oled_module.draw(im);
        // for key in &keys[2..8] {
        //     let c = (key - 0x1e + '1' as u8) as char;
        //     self.oled_module.draw_char(c);
        // }
        // self.oled_module.flush();
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
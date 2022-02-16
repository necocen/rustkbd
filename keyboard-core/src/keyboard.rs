use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::Point,
};
use embedded_hal::blocking::delay::DelayMs;

use crate::{
    key_switches::KeySwitches, oled::OledModule, usart::UsartController, usb::UsbController,
};

pub struct Keyboard<K: KeySwitches, U: UsbController, V: UsartController, O: OledModule> {
    key_switches: K,
    usb_controller: U,
    usart_controller: V,
    oled_module: O,
}

impl<
        K: KeySwitches<Identifier = (u8, u8)>,
        U: UsbController,
        V: UsartController<KeySwitchId = (u8, u8)>,
        O: OledModule,
    > Keyboard<K, U, V, O>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];
    const RUST_LOGO: ImageRaw<'static, BinaryColor> =
        ImageRaw::new_binary(include_bytes!("./rust.raw"), 64);
    pub fn new(key_switches: K, usb_controller: U, usart_controller: V, oled_module: O) -> Self {
        Keyboard {
            key_switches,
            usb_controller,
            usart_controller,
            oled_module,
        }
    }

    pub fn main_loop(&mut self, delay: &mut impl DelayMs<u16>) -> ! {
        // let im = Image::new(&Self::RUST_LOGO, Point::new(0, 0));
        loop {
            let left = self.key_switches.scan();
            self.usart_controller.put(&left);
            let right = self.usart_controller.get();
            let keys = self.key_codes(&left, right.as_deref());
            // FIXME: USBConはそっちでIDLEにあわせて送信する必要があるので、それも含めて検討が必要であろう
            self.usb_controller.send(keys);
            self.oled_module.set_cursor(0, 0);
            self.oled_module.clear();
            //self.oled_module.draw(im);
            for key in &keys[2..8] {
                let c = (key - 0x1e + '1' as u8) as char;
                self.oled_module.draw_char(c);
            }
            self.oled_module.flush();
            delay.delay_ms(24u16);
        }
    }

    fn key_codes(&self, left: &[(u8, u8)], right: Option<&[(u8, u8)]>) -> [u8; 8] {
        let mut keys = [0u8; 8];
        let mut i = 2;
        for (col, row) in left.iter() {
            keys[i] = Self::KEY_CODES_LEFT[*col as usize][*row as usize];
            i += 1;
        }
        if let Some(others) = right {
            for (col, row) in others.iter() {
                keys[i] = Self::KEY_CODES_RIGHT[*col as usize][*row as usize];
                i += 1;
            }
        }
        keys
    }
}

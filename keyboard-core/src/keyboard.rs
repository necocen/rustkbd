use embedded_graphics::{
    draw_target::DrawTarget,
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::Point,
    Drawable,
};
use embedded_hal::blocking::delay::DelayMs;

use crate::{key_switches::KeySwitches, usart::UsartController, usb::UsbController};

pub struct Keyboard<K: KeySwitches, U: UsbController, V: UsartController, D: DrawTarget> {
    key_switches: K,
    usb_controller: U,
    usart_controller: V,
    draw_target: D,
}

impl<
        K: KeySwitches<Identifier = (u8, u8)>,
        U: UsbController,
        V: UsartController<KeySwitchId = (u8, u8)>,
        D: DrawTarget<Color = BinaryColor>,
    > Keyboard<K, U, V, D>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];
    pub fn new(key_switches: K, usb_controller: U, usart_controller: V, draw_target: D) -> Self {
        Keyboard {
            key_switches,
            usb_controller,
            usart_controller,
            draw_target,
        }
    }

    pub fn main_loop(&mut self, delay: &mut impl DelayMs<u16>) -> ! {
        let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("./rust.raw"), 64);
        let im = Image::new(&raw, Point::new(0, 0));
        //self.oled_module.draw_image(im);
        im.draw(&mut self.draw_target).ok();
        loop {
            let left = self.key_switches.scan();
            self.usart_controller.put(&left);
            let right = self.usart_controller.get();
            let keys = self.key_codes(&left, right.as_deref());
            // FIXME: USBConはそっちでIDLEにあわせて送信する必要があるので、それも含めて検討が必要であろう
            self.usb_controller.send(keys);
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

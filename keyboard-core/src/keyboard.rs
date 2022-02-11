use embedded_hal::blocking::delay::DelayMs;

use crate::{key_switches::KeySwitches, usb::UsbController};

pub struct Keyboard<K: KeySwitches, U: UsbController> {
    key_switches: K,
    usb_controller: U,
}

impl<K: KeySwitches<Identifier = (u8, u8)>, U: UsbController> Keyboard<K, U> {
    const KEY_CODES: [[u8; 2]; 2] = [[0x11, 0x08], [0x06, 0x12]];
    pub fn new(key_switches: K, usb_controller: U) -> Self {
        Keyboard {
            key_switches,
            usb_controller,
        }
    }

    pub fn main_loop(&self, delay: &mut impl DelayMs<u16>) -> ! {
        loop {
            let mut keys = [0u8; 8];
            let mut i = 2;
            for (col, row) in self.key_switches.scan().into_iter() {
                keys[i] = Self::KEY_CODES[row as usize][col as usize];
                i += 1;
            }
            // FIXME: USBConはそっちでIDLEにあわせて送信する必要があるので、それも含めて検討が必要であろう
            self.usb_controller.send(keys);
            delay.delay_ms(24u16);
        }
    }
}

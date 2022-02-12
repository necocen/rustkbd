use embedded_hal::blocking::delay::DelayMs;

use crate::{key_switches::KeySwitches, usart::UsartController, usb::UsbController};

pub struct Keyboard<K: KeySwitches, U: UsbController, V: UsartController> {
    key_switches: K,
    usb_controller: U,
    usart_controller: V,
}

impl<
        K: KeySwitches<Identifier = (u8, u8)>,
        U: UsbController,
        V: UsartController<KeySwitchId = (u8, u8)>,
    > Keyboard<K, U, V>
{
    const KEY_CODES: [[u8; 2]; 4] = [[0x1e, 0x1f], [0x20, 0x21], [0x22, 0x23], [0x24, 0x25]];
    pub fn new(key_switches: K, usb_controller: U, usart_controller: V) -> Self {
        Keyboard {
            key_switches,
            usb_controller,
            usart_controller,
        }
    }

    pub fn main_loop(&mut self, delay: &mut impl DelayMs<u16>) -> ! {
        loop {
            let mut keys = [0u8; 8];
            let mut i = 2;
            let scan = self.key_switches.scan();
            self.usart_controller.put(&scan);
            for (col, row) in scan.iter() {
                keys[i] = Self::KEY_CODES[*col as usize][*row as usize];
                i += 1;
            }
            if let Some(others) = self.usart_controller.get() {
                for (col, row) in others.iter() {
                    keys[i] =
                        Self::KEY_CODES[*col as usize + Self::KEY_CODES.len() / 2][*row as usize];
                    i += 1;
                }
            }
            // FIXME: USBConはそっちでIDLEにあわせて送信する必要があるので、それも含めて検討が必要であろう
            self.usb_controller.send(keys);
            delay.delay_ms(24u16);
        }
    }
}

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use atmega_hal::{clock::MHz16, delay::Delay, pins, Peripherals};
use avr_device::interrupt;
use keyboard_core::keyboard::Keyboard;
use oled::init_display;
use pro_micro_usart::ProMicroUsart;
use usb::pro_micro_usb::ProMicroUsb;

use crate::key_matrix::KeyMatrix;

mod key_matrix;
mod oled;
mod pro_micro_usart;

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let usb = ProMicroUsb::new(dp.USB_DEVICE, dp.PLL);
    let usart = ProMicroUsart::new(dp.USART1);
    let pins = pins!(dp);
    let b2 = pins.pb2.into_pull_up_input().downgrade();
    let b6 = pins.pb6.into_pull_up_input().downgrade();
    let b4 = pins.pb4.into_output_high().downgrade();
    let b5 = pins.pb5.into_output_high().downgrade();
    let mut delay = Delay::<MHz16>::new();
    let key_matrix = KeyMatrix::new([b2, b6], [b4, b5]);
    let oled_module = init_display(
        dp.TWI,
        pins.pd1.into_pull_up_input(),
        pins.pd0.into_pull_up_input(),
    );
    let mut keyboard = Keyboard::new(key_matrix, usb, usart, oled_module);

    unsafe {
        interrupt::enable();
    }

    keyboard.main_loop(&mut delay);
}

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use atmega_hal::{clock::MHz16, delay::Delay, pins, I2c, Peripherals};
use avr_device::interrupt;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::Point,
    prelude::*,
};
use keyboard_core::keyboard::Keyboard;
use pro_micro_usart::ProMicroUsart;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use usb::pro_micro_usb::ProMicroUsb;

use crate::key_matrix::KeyMatrix;

mod key_matrix;
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
    let mut keyboard = Keyboard::new(key_matrix, usb, usart);
    let i2c = I2c::<MHz16>::new(
        dp.TWI,
        pins.pd1.into_pull_up_input(),
        pins.pd0.into_pull_up_input(),
        51200,
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let raw: ImageRaw<BinaryColor> = ImageRaw::new(include_bytes!("./rust.raw"), 64);
    let im = Image::new(&raw, Point::new(32, 0));
    im.draw(&mut display).unwrap();
    display.flush().unwrap();

    unsafe {
        interrupt::enable();
    }

    keyboard.main_loop(&mut delay);
}

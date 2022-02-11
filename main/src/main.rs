#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use core::cell::RefCell;

use atmega_hal::{
    clock::MHz16,
    delay::Delay,
    pins,
    port::{
        mode::{Input, Output, PullUp},
        Dynamic, Pin,
    },
    I2c, Peripherals,
};
use avr_device::interrupt;
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::Point,
    prelude::*,
};
use embedded_hal::blocking::delay::DelayMs;
use heapless::Vec;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use usb::usb_device::UsbDevice;

static KEY_CODES: [[u8; 2]; 2] = [[0x11, 0x08], [0x06, 0x12]];

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let usb = UsbDevice::new(dp.USB_DEVICE, dp.PLL);
    let pins = pins!(dp);
    let b2 = pins.pb2.into_pull_up_input().downgrade();
    let b6 = pins.pb6.into_pull_up_input().downgrade();
    let b4 = pins.pb4.into_output_high().downgrade();
    let b5 = pins.pb5.into_output_high().downgrade();
    let mut delay = Delay::<MHz16>::new();
    let keyboard = KeyMatrix::new([b2, b6], [b4, b5]);
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

    loop {
        let mut keys = [0u8; 8];
        let mut i = 2;
        for (col, row) in keyboard.scan().into_iter() {
            keys[i] = KEY_CODES[row as usize][col as usize];
            i += 1;
        }
        usb.send(keys);
        // FIXME: idleを適切に設定する方法がない
        //let idle = (KEYBOARD_IDLE_VALUE.load(Ordering::Relaxed) as u16) << 2;
        delay.delay_ms(24u16);
    }
}

trait KeySwitches {
    type Identifier: Copy + Sized;
    fn scan(&self) -> Vec<Self::Identifier, 6>;
}

struct KeyMatrix<const COLS: usize, const ROWS: usize> {
    inputs: [Pin<Input<PullUp>, Dynamic>; ROWS],
    outputs: RefCell<[Pin<Output, Dynamic>; COLS]>,
}

impl<const COLS: usize, const ROWS: usize> KeyMatrix<COLS, ROWS> {
    pub fn new(
        inputs: [Pin<Input<PullUp>, Dynamic>; ROWS],
        outputs: [Pin<Output, Dynamic>; COLS],
    ) -> Self {
        KeyMatrix {
            inputs,
            outputs: RefCell::new(outputs),
        }
    }
}

impl<const COLS: usize, const ROWS: usize> KeySwitches for KeyMatrix<COLS, ROWS> {
    type Identifier = (u8, u8);

    fn scan(&self) -> Vec<Self::Identifier, 6> {
        let mut keys = Vec::<Self::Identifier, 6>::new();
        let mut outputs = self.outputs.borrow_mut();

        for i in 0..COLS {
            outputs[i].set_low();
            for j in 0..ROWS {
                if self.inputs[j].is_low() {
                    keys.push((i as u8, j as u8)).ok();
                }
            }
            outputs[i].set_high();
        }
        keys
    }
}

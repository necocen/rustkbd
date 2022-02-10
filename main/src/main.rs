#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use atmega_hal::{
    clock::MHz16,
    delay::Delay,
    pins,
    port::{
        mode::{Input, Output, PullUp},
        Dynamic, Pin,
    },
    Peripherals,
};
use avr_device::interrupt;
use embedded_hal::blocking::delay::DelayMs;
use usb::usb_device::UsbDevice;

static KEY_CODES: [[u8; 2]; 2] = [[0x11, 0x08], [0x06, 0x12]];

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let usb = UsbDevice::new(dp.USB_DEVICE, dp.PLL);
    let pins = pins!(dp);
    let b2 = pins.pb2.into_pull_up_input().downgrade();
    let b6 = pins.pb6.into_pull_up_input().downgrade();
    let mut b4 = pins.pb4.into_output_high().downgrade();
    let mut b5 = pins.pb5.into_output_high().downgrade();
    let mut delay = Delay::<MHz16>::new();

    unsafe {
        interrupt::enable();
    }

    let mut keys = [0u8; 8];
    loop {
        matrix_scan(&b2, &mut b4, &mut b5, &b6, &mut keys);
        usb.send(keys);
        // FIXME: idleを適切に設定する方法がない
        //let idle = (KEYBOARD_IDLE_VALUE.load(Ordering::Relaxed) as u16) << 2;
        delay.delay_ms(24u16);
    }
}

fn matrix_scan(
    b2: &Pin<Input<PullUp>, Dynamic>,
    b4: &mut Pin<Output, Dynamic>,
    b5: &mut Pin<Output, Dynamic>,
    b6: &Pin<Input<PullUp>, Dynamic>,
    keys: &mut [u8; 8],
) {
    let input = [b2, b6];
    let output = [b4, b5];
    let mut index = 2usize;
    for key in keys.iter_mut() {
        *key = 0;
    }
    for i in 0..2 {
        output[i].set_low();
        for j in 0..2 {
            if input[j].is_low() {
                keys[index] = KEY_CODES[i][j];
                index += 1;
            }
        }
        output[i].set_high();
    }
}

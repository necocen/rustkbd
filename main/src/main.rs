#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use core::cell::RefCell;

use atmega_hal::{
    clock::MHz16,
    delay::Delay,
    pins,
    port::{mode::Input, Pin, PB1},
    Peripherals,
};
use avr_device::interrupt::{self, Mutex};
use embedded_hal::blocking::delay::DelayMs;
use usb::usb_device::UsbDevice;

static MY_B1: Mutex<RefCell<Option<Pin<Input, PB1>>>> = Mutex::new(RefCell::new(None));

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let usb = UsbDevice::new(dp.USB_DEVICE, dp.PLL);
    let pins = pins!(dp);
    let mut delay = Delay::<MHz16>::new();

    interrupt::free(|cs| {
        MY_B1
            .borrow(cs)
            .replace(Some(pins.pb1.into_pull_up_input().forget_imode()));
    });

    unsafe {
        interrupt::enable();
    }
    loop {
        let b1_is_down = interrupt::free(|cs| {
            let b1 = MY_B1.borrow(cs).borrow();
            let b1 = b1.as_ref().unwrap();
            b1.is_low()
        });
        usb.send([0, 0, if b1_is_down { 4 } else { 0 }, 0, 0, 0, 0, 0]);
        // FIXME: idleを適切に設定する方法がない
        //let idle = (KEYBOARD_IDLE_VALUE.load(Ordering::Relaxed) as u16) << 2;
        delay.delay_ms(24u16);
    }
}

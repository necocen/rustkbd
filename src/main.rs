#![no_std]
#![no_main]
extern crate panic_halt;

use atmega_hal::{clock, pins, Peripherals};

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let pins = pins!(dp);
    let mut leds = [
        pins.pb0.into_output().downgrade(), // PB0: RX LED
        pins.pd5.into_output().downgrade(), // PD5: TX LED
    ];

    leds[0].set_high();
    leds[1].set_low();

    loop {
        for i in 0..2 {
            leds[i].toggle();
        }

        // 0.1秒待機
        avr_delay::delay_ms(100);
    }
}

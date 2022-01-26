#![no_std]
#![no_main]
extern crate panic_halt;

use avr_delay::delay_ms;
use avr_device::atmega32u4::PORTD;

#[no_mangle]
pub extern "C" fn main() {
    // PB0: RX LED
    // PD5: TX LED
    // 以下はPD5を点滅させる
    let ptr = PORTD::ptr();

    unsafe {
        // DDR: データの流れの向きを設定する。1だとPD5が出力になる
        (*ptr).ddrd.modify(|_, w| w.pd5().set_bit());
        // PD5を落としておく
        (*ptr).portd.write(|w| w.pd5().clear_bit())
    }

    loop {
        // PD5をオン
        unsafe { (*ptr).portd.write(|w| w.pd5().set_bit()) }
        // 0.1秒待機
        delay_ms(100);
        // PD5をオフ
        unsafe { (*ptr).portd.write(|w| w.pd5().clear_bit()) }
        // 0.1秒待機
        delay_ms(100);
    }
}

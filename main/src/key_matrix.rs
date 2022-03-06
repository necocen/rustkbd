use core::cell::RefCell;

use embedded_hal::{
    blocking::delay::DelayUs,
    digital::v2::{InputPin, OutputPin},
};
use heapless::Vec;
use keyboard_core::key_switches::KeySwitches;
use rp_pico::hal::gpio::DynPin;

pub struct KeyMatrix<D: DelayUs<u16>, const COLS: usize, const ROWS: usize> {
    inputs: [DynPin; ROWS],
    outputs: RefCell<[DynPin; COLS]>,
    delay: RefCell<D>,
}

impl<D: DelayUs<u16>, const COLS: usize, const ROWS: usize> KeyMatrix<D, COLS, ROWS> {
    pub fn new(mut inputs: [DynPin; ROWS], mut outputs: [DynPin; COLS], delay: D) -> Self {
        for pin in inputs.iter_mut() {
            pin.into_pull_up_input();
        }
        for pin in outputs.iter_mut() {
            pin.into_push_pull_output();
            pin.set_high().ok();
        }
        KeyMatrix {
            inputs,
            outputs: RefCell::new(outputs),
            delay: RefCell::new(delay),
        }
    }
}

impl<D: DelayUs<u16>, const COLS: usize, const ROWS: usize> KeySwitches
    for KeyMatrix<D, COLS, ROWS>
{
    type Identifier = (u8, u8);

    fn scan(&self) -> Vec<Self::Identifier, 6> {
        let mut keys = Vec::<Self::Identifier, 6>::new();
        let mut outputs = self.outputs.borrow_mut();
        for i in 0..COLS {
            outputs[i].set_low().ok();
            self.delay.borrow_mut().delay_us(20);
            for j in 0..ROWS {
                if self.inputs[j].is_low().unwrap() {
                    keys.push((i as u8, j as u8)).ok();
                }
            }
            outputs[i].set_high().ok();
        }
        keys
    }
}

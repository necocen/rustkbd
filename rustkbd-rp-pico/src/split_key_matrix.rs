use core::cell::RefCell;

use embedded_hal::{
    blocking::delay::DelayUs,
    digital::v2::{InputPin, OutputPin},
};
use heapless::Vec;
use rp_pico::hal::gpio::DynPin;
use rustkbd_core::keyboard::KeySwitches;

use crate::split_switch_identifier::SplitKeySwitchIdentifier;

pub struct SplitKeyMatrix<D: DelayUs<u16>, const COLS: usize, const ROWS: usize> {
    inputs: [DynPin; ROWS],
    outputs: RefCell<[DynPin; COLS]>,
    delay: RefCell<D>,
    is_left: bool,
}

impl<D: DelayUs<u16>, const COLS: usize, const ROWS: usize> SplitKeyMatrix<D, COLS, ROWS> {
    pub fn new(
        mut inputs: [DynPin; ROWS],
        mut outputs: [DynPin; COLS],
        delay: D,
        is_left: bool,
    ) -> Self {
        for pin in inputs.iter_mut() {
            pin.into_pull_down_input();
        }
        for pin in outputs.iter_mut() {
            pin.into_push_pull_output();
            pin.set_low().ok();
        }
        SplitKeyMatrix {
            inputs,
            outputs: RefCell::new(outputs),
            delay: RefCell::new(delay),
            is_left,
        }
    }
}

impl<D: DelayUs<u16>, const COLS: usize, const ROWS: usize> KeySwitches<3, 12>
    for SplitKeyMatrix<D, COLS, ROWS>
{
    type Identifier = SplitKeySwitchIdentifier;

    fn scan(&self) -> Vec<Self::Identifier, 12> {
        let mut keys = Vec::<Self::Identifier, 12>::new();
        let mut outputs = self.outputs.borrow_mut();
        for i in 0..COLS {
            outputs[i].set_high().ok();
            self.delay.borrow_mut().delay_us(20);
            for j in 0..ROWS {
                if self.inputs[j].is_high().unwrap() {
                    if self.is_left {
                        keys.push(SplitKeySwitchIdentifier::Left(i as u8, j as u8))
                            .ok();
                    } else {
                        keys.push(SplitKeySwitchIdentifier::Right(i as u8, j as u8))
                            .ok();
                    }
                }
            }
            outputs[i].set_low().ok();
        }
        keys
    }
}

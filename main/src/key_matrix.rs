use core::cell::RefCell;

use heapless::Vec;
use keyboard_core::key_switches::KeySwitches;
use rp_pico::hal::gpio::{Input, Output, PullUp, PushPull, Readable, SpecificPin};

pub struct KeyMatrix<const COLS: usize, const ROWS: usize> {
    inputs: [SpecificPin<Input<PullUp>>; ROWS],
    outputs: RefCell<[SpecificPin<Output<PushPull>>; COLS]>,
}

impl<const COLS: usize, const ROWS: usize> KeyMatrix<COLS, ROWS> {
    pub fn new(
        inputs: [SpecificPin<Input<PullUp>>; ROWS],
        outputs: [SpecificPin<Output<Readable>>; COLS],
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

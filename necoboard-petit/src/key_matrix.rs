use embedded_hal::{
    blocking::delay::DelayUs,
    digital::v2::{InputPin, OutputPin},
};
use rp_pico::hal::gpio::DynPin;
use rustkbd::{keyboard, Vec};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeySwitchIdentifier {
    pub row: u8,
    pub col: u8,
}

impl From<[u8; 2]> for KeySwitchIdentifier {
    fn from(value: [u8; 2]) -> Self {
        KeySwitchIdentifier {
            row: value[0],
            col: value[1],
        }
    }
}

impl From<KeySwitchIdentifier> for [u8; 2] {
    fn from(value: KeySwitchIdentifier) -> Self {
        [value.row, value.col]
    }
}

impl keyboard::KeySwitchIdentifier<2> for KeySwitchIdentifier {}

pub struct KeyMatrix<D: DelayUs<u16>, const ROWS: usize, const COLS: usize> {
    inputs: [DynPin; ROWS],
    outputs: [DynPin; COLS],
    delay: D,
}

impl<D: DelayUs<u16>, const ROWS: usize, const COLS: usize> KeyMatrix<D, ROWS, COLS> {
    pub fn new(mut inputs: [DynPin; ROWS], mut outputs: [DynPin; COLS], delay: D) -> Self {
        for pin in inputs.iter_mut() {
            pin.into_pull_down_input();
        }
        for pin in outputs.iter_mut() {
            pin.into_push_pull_output();
            pin.set_low().ok();
        }
        KeyMatrix {
            inputs,
            outputs,
            delay,
        }
    }
}

impl<D: DelayUs<u16>, const ROWS: usize, const COLS: usize> keyboard::KeySwitches<2, 12>
    for KeyMatrix<D, ROWS, COLS>
{
    type Identifier = KeySwitchIdentifier;

    fn scan(&mut self) -> Vec<Self::Identifier, 12> {
        let mut keys = Vec::<Self::Identifier, 12>::new();
        for i in 0..COLS {
            self.outputs[i].set_high().ok();
            self.delay.delay_us(20);
            for j in 0..ROWS {
                if self.inputs[j].is_high().unwrap() {
                    keys.push(KeySwitchIdentifier {
                        row: j as u8,
                        col: i as u8,
                    })
                    .ok();
                }
            }
            self.outputs[i].set_low().ok();
        }
        keys
    }
}

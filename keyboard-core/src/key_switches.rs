use embedded_hal::blocking::delay::DelayUs;
use heapless::Vec;

pub trait KeySwitches {
    type Identifier: Copy + Sized;
    fn scan(&self, delay: &mut impl DelayUs<u16>) -> Vec<Self::Identifier, 6>;
}

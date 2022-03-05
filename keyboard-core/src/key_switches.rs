use heapless::Vec;

pub trait KeySwitches {
    type Identifier: Copy + Sized;
    fn scan(&self) -> Vec<Self::Identifier, 6>;
}

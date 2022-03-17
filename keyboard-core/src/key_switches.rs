use heapless::Vec;

pub trait KeySwitches<const SIZE: usize> {
    type Identifier: KeySwitchIdentifier<SIZE>;
    fn scan(&self) -> Vec<Self::Identifier, 6>;
}

pub trait KeySwitchIdentifier<const SIZE: usize>:
    Copy + Sized + Eq + From<[u8; SIZE]> + Into<[u8; SIZE]>
{
}

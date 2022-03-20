use heapless::Vec;

pub trait KeySwitches<const SZ: usize, const RO: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;
    fn scan(&self) -> Vec<Self::Identifier, RO>;
}

pub trait KeySwitchIdentifier<const SZ: usize>:
    Copy + Eq + From<[u8; SZ]> + Into<[u8; SZ]>
{
}

use core::hash::Hash;

use crate::Vec;

pub trait KeySwitches<const SZ: usize, const RO: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;
    fn scan(&mut self) -> Vec<Self::Identifier, RO>;
}

pub trait KeySwitchIdentifier<const SZ: usize>:
    Copy + Eq + From<[u8; SZ]> + Into<[u8; SZ]> + Hash
{
}

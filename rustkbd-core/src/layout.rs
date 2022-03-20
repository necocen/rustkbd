use heapless::Vec;

use crate::keyboard::{Key, KeySwitchIdentifier};

pub trait Layout<const SZ: usize, const RO: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn keys(&self, switches: &[Self::Identifier]) -> Vec<Key, RO>;
}

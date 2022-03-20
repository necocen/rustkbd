use crate::keyboard::{Key, KeySwitchIdentifier};

pub trait Layout<const SZ: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn key(&self, switch: Self::Identifier) -> Option<Key>;
}

use crate::keyboard::{Key, KeySwitchIdentifier, KeyboardLayer};

pub trait Layout<const SZ: usize, L: KeyboardLayer> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn layer(&self, switches: &[Self::Identifier]) -> L;

    fn key(&self, layer: L, switch: &Self::Identifier) -> Key;
}

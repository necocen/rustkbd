use crate::keyboard::{Key, KeySwitchIdentifier, Layer};

pub trait Layout<const SZ: usize, L: Layer> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn layer(&self, switches: &[Self::Identifier]) -> L;

    fn key(&self, layer: L, switch: &Self::Identifier) -> Key;
}

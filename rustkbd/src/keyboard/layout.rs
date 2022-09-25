use crate::keyboard::{Key, KeySwitchIdentifier, Layer};

pub trait Layout<const SZ: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;
    type Layer: Layer;

    fn layer(&self, switches: &[Self::Identifier]) -> Self::Layer;

    fn key(&self, layer: Self::Layer, switch: &Self::Identifier) -> Key;
}

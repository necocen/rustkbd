use crate::{key_switches::KeySwitchIdentifier, keyboard::NUM_ROLLOVER};

pub trait KeyLayout<const SZ: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn key_codes(&self, switches: &[Self::Identifier]) -> [u8; NUM_ROLLOVER];
}

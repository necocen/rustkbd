use crate::keyboard::KeySwitchIdentifier;

pub trait Layout<const SZ: usize, const RO: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn key_codes(&self, switches: &[Self::Identifier]) -> [u8; RO];
}

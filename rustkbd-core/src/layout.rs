use crate::key_switches::KeySwitchIdentifier;

pub trait KeyLayout<const SZ: usize, const RO: usize> {
    type Identifier: KeySwitchIdentifier<SZ>;

    fn key_codes(&self, switches: &[Self::Identifier]) -> [u8; RO];
}

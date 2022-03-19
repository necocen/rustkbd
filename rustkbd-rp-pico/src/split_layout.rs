use rustkbd_core::layout::KeyLayout;

use crate::split_switch_identifier::SplitKeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SplitLayout {}

impl SplitLayout {
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];
}

impl KeyLayout<3, 6> for SplitLayout {
    type Identifier = SplitKeySwitchIdentifier;

    fn key_codes(&self, switches: &[Self::Identifier]) -> [u8; 6] {
        let mut keys = [0u8; 6];
        switches
            .iter()
            .map(|switch| match *switch {
                SplitKeySwitchIdentifier::Left(col, row) => {
                    Self::KEY_CODES_LEFT[col as usize][row as usize]
                }
                SplitKeySwitchIdentifier::Right(col, row) => {
                    Self::KEY_CODES_RIGHT[col as usize][row as usize]
                }
            })
            .take(6)
            .enumerate()
            .for_each(|(i, key)| keys[i] = key);
        keys
    }
}

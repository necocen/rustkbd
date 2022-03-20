use rustkbd_core::{keyboard::Key, layout::Layout};

use crate::split_switch_identifier::SplitKeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SplitLayout {}

impl SplitLayout {
    const KEY_CODES_LEFT: [[Key; 2]; 2] = [
        [Key::Digit1_Exclamation, Key::Digit2_At],
        [Key::Digit3_Number, Key::Digit4_Dollar],
    ];
    const KEY_CODES_RIGHT: [[Key; 2]; 2] = [
        [Key::Digit5_Percent, Key::Digit6_Circumflex],
        [Key::Digit7_Ampersand, Key::LeftShift],
    ];
}

impl Layout<3> for SplitLayout {
    type Identifier = SplitKeySwitchIdentifier;

    fn key(&self, switch: Self::Identifier) -> Option<Key> {
        let key = match switch {
            SplitKeySwitchIdentifier::Left(col, row) => {
                Self::KEY_CODES_LEFT[col as usize][row as usize]
            }
            SplitKeySwitchIdentifier::Right(col, row) => {
                Self::KEY_CODES_RIGHT[col as usize][row as usize]
            }
        };
        Some(key)
    }
}

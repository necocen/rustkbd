use rustkbd_core::{
    keyboard::{Key, KeyboardLayer},
    layout::Layout,
};

use crate::split_switch_identifier::SplitKeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SplitLayout {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, KeyboardLayer)]
pub enum Layer {
    Default,
    Lower,
    Raise,
}

impl Default for Layer {
    fn default() -> Self {
        Self::Default
    }
}

impl SplitLayout {
    const KEY_CODES_LEFT: [[Key; 2]; 2] = [
        [Key::Digit1_Exclamation, Key::Digit2_At],
        [Key::LeftShift, Key::Delete],
    ];
    const KEY_CODES_RIGHT: [[Key; 2]; 2] = [
        [Key::Digit3_Number, Key::Digit4_Dollar],
        [Key::None, Key::None],
    ];
    const KEY_CODES_LOWER_LEFT: [[Key; 2]; 2] =
        [[Key::A, Key::B], [Key::Transparent, Key::Transparent]];
    const KEY_CODES_LOWER_RIGHT: [[Key; 2]; 2] = [[Key::C, Key::D], [Key::None, Key::None]];
    const KEY_CODES_RAISE_LEFT: [[Key; 2]; 2] = [
        [Key::None, Key::MediaVolumeDecrement],
        [Key::None, Key::None],
    ];
    const KEY_CODES_RAISE_RIGHT: [[Key; 2]; 2] = [
        [Key::MediaPlayPause, Key::MediaVolumeIncrement],
        [Key::None, Key::None],
    ];
}

impl Layout<3, Layer> for SplitLayout {
    type Identifier = SplitKeySwitchIdentifier;

    fn layer(&self, switches: &[Self::Identifier]) -> Layer {
        switches
            .iter()
            .map(|switch| match switch {
                SplitKeySwitchIdentifier::Right { row: 1, col: 0 } => Layer::Lower,
                SplitKeySwitchIdentifier::Right { row: 1, col: 1 } => Layer::Raise,
                _ => Layer::Default,
            })
            .max()
            .unwrap_or_default()
    }

    fn key(&self, layer: Layer, switch: Self::Identifier) -> Key {
        match (layer, switch) {
            (Layer::Default, SplitKeySwitchIdentifier::Left { row, col }) => {
                Self::KEY_CODES_LEFT[row as usize][col as usize]
            }
            (Layer::Default, SplitKeySwitchIdentifier::Right { row, col }) => {
                Self::KEY_CODES_RIGHT[row as usize][col as usize]
            }
            (Layer::Lower, SplitKeySwitchIdentifier::Left { row, col }) => {
                Self::KEY_CODES_LOWER_LEFT[row as usize][col as usize]
            }
            (Layer::Lower, SplitKeySwitchIdentifier::Right { row, col }) => {
                Self::KEY_CODES_LOWER_RIGHT[row as usize][col as usize]
            }
            (Layer::Raise, SplitKeySwitchIdentifier::Left { row, col }) => {
                Self::KEY_CODES_RAISE_LEFT[row as usize][col as usize]
            }
            (Layer::Raise, SplitKeySwitchIdentifier::Right { row, col }) => {
                Self::KEY_CODES_RAISE_RIGHT[row as usize][col as usize]
            }
        }
    }
}

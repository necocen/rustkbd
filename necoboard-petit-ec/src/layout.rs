use rustkbd_core::keyboard::{Key, KeyboardLayer};

use crate::switch_identifier::KeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Layout {}

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

impl Layout {
    const KEY_CODES_LEFT: [[Key; 3]; 3] = [
        [Key::Digit1_Exclamation, Key::Digit2_At, Key::Digit3_Number],
        [
            Key::Digit4_Dollar,
            Key::Digit5_Percent,
            Key::Digit6_Circumflex,
        ],
        [
            Key::Digit7_Ampersand,
            Key::Digit8_Asterisk,
            Key::Digit9_LeftParenthesis,
        ],
    ];
    const KEY_CODES_RIGHT: [[Key; 3]; 3] = [
        [Key::Digit0_RightParenthesis, Key::LeftShift, Key::Delete],
        [Key::RightShift, Key::RightControl, Key::Enter],
        [Key::Space, Key::None, Key::None],
    ];
    const KEY_CODES_LOWER_LEFT: [[Key; 3]; 3] = [
        [Key::A, Key::B, Key::C],
        [Key::D, Key::E, Key::F],
        [Key::G, Key::H, Key::I],
    ];
    const KEY_CODES_LOWER_RIGHT: [[Key; 3]; 3] = [
        [Key::J, Key::Transparent, Key::Transparent],
        [Key::Transparent, Key::Transparent, Key::Transparent],
        [Key::Transparent, Key::None, Key::None],
    ];
    const KEY_CODES_RAISE_LEFT: [[Key; 3]; 3] = [
        [
            Key::MediaVolumeDecrement,
            Key::MediaMute,
            Key::MediaVolumeIncrement,
        ],
        [
            Key::MediaPrevTrack,
            Key::MediaPlayPause,
            Key::MediaNextTrack,
        ],
        [Key::None, Key::None, Key::None],
    ];
    const KEY_CODES_RAISE_RIGHT: [[Key; 3]; 3] = [
        [Key::None, Key::UpArrow, Key::None],
        [Key::LeftArrow, Key::DownArrow, Key::RightArrow],
        [Key::None, Key::None, Key::None],
    ];
}

impl rustkbd_core::layout::Layout<3, Layer> for Layout {
    type Identifier = KeySwitchIdentifier;

    fn layer(&self, switches: &[Self::Identifier]) -> Layer {
        switches
            .iter()
            .map(|switch| match switch {
                KeySwitchIdentifier::Right { row: 2, col: 1 } => Layer::Lower,
                KeySwitchIdentifier::Right { row: 2, col: 2 } => Layer::Raise,
                _ => Layer::Default,
            })
            .max()
            .unwrap_or_default()
    }

    fn key(&self, layer: Layer, switch: Self::Identifier) -> Key {
        match (layer, switch) {
            (Layer::Default, KeySwitchIdentifier::Left { row, col }) => {
                Self::KEY_CODES_LEFT[row as usize][col as usize]
            }
            (Layer::Default, KeySwitchIdentifier::Right { row, col }) => {
                Self::KEY_CODES_RIGHT[row as usize][col as usize]
            }
            (Layer::Lower, KeySwitchIdentifier::Left { row, col }) => {
                Self::KEY_CODES_LOWER_LEFT[row as usize][col as usize]
            }
            (Layer::Lower, KeySwitchIdentifier::Right { row, col }) => {
                Self::KEY_CODES_LOWER_RIGHT[row as usize][col as usize]
            }
            (Layer::Raise, KeySwitchIdentifier::Left { row, col }) => {
                Self::KEY_CODES_RAISE_LEFT[row as usize][col as usize]
            }
            (Layer::Raise, KeySwitchIdentifier::Right { row, col }) => {
                Self::KEY_CODES_RAISE_RIGHT[row as usize][col as usize]
            }
        }
    }
}

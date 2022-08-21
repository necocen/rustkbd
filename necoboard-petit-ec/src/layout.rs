use rustkbd::keyboard::{self, Key};

use crate::switch_identifier::KeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Layout {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, keyboard::Layer)]
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
    const KEY_CODES_DEFAULT: [[Key; 4]; 4] = [
        [
            Key::Digit1_Exclamation,
            Key::Digit2_At,
            Key::Digit3_Number,
            Key::Digit4_Dollar,
        ],
        [
            Key::Digit5_Percent,
            Key::Digit6_Circumflex,
            Key::Digit7_Ampersand,
            Key::Digit8_Asterisk,
        ],
        [
            Key::Digit9_LeftParenthesis,
            Key::Digit0_RightParenthesis,
            Key::Delete,
            Key::Enter,
        ],
        [Key::None, Key::None, Key::None, Key::None],
    ];
    const KEY_CODES_LOWER: [[Key; 4]; 4] = [
        [Key::A, Key::B, Key::C, Key::D],
        [Key::E, Key::F, Key::G, Key::H],
        [Key::I, Key::J, Key::K, Key::L],
        [Key::None, Key::None, Key::None, Key::None],
    ];
    const KEY_CODES_RAISE: [[Key; 4]; 4] = [
        [Key::M, Key::N, Key::O, Key::P],
        [Key::Q, Key::R, Key::S, Key::T],
        [Key::U, Key::V, Key::W, Key::X],
        [Key::None, Key::None, Key::None, Key::None],
    ];
}

impl rustkbd::keyboard::Layout<2, Layer> for Layout {
    type Identifier = KeySwitchIdentifier;

    fn layer(&self, switches: &[Self::Identifier]) -> Layer {
        switches
            .iter()
            .map(|switch| match switch {
                KeySwitchIdentifier { row: 3, col: 2 } => Layer::Lower,
                KeySwitchIdentifier { row: 3, col: 3 } => Layer::Raise,
                _ => Layer::Default,
            })
            .max()
            .unwrap_or_default()
    }

    fn key(&self, layer: Layer, switch: &Self::Identifier) -> Key {
        match (layer, *switch) {
            (Layer::Default, KeySwitchIdentifier { row, col }) => {
                Self::KEY_CODES_DEFAULT[row as usize][col as usize]
            }
            (Layer::Lower, KeySwitchIdentifier { row, col }) => {
                Self::KEY_CODES_LOWER[row as usize][col as usize]
            }
            (Layer::Raise, KeySwitchIdentifier { row, col }) => {
                Self::KEY_CODES_RAISE[row as usize][col as usize]
            }
        }
    }
}

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
    const KEY_CODES_DEFAULT: [[Key; 12]; 4] = [
        [
            Key::Escape,
            Key::Q,
            Key::W,
            Key::E,
            Key::R,
            Key::T,
            Key::Y,
            Key::U,
            Key::I,
            Key::O,
            Key::P,
            Key::Delete,
        ],
        [
            Key::LeftControl,
            Key::A,
            Key::S,
            Key::D,
            Key::F,
            Key::G,
            Key::H,
            Key::J,
            Key::K,
            Key::L,
            Key::Semicolon_Colon,
            Key::Apostrophe_Quotation,
        ],
        [
            Key::LeftShift,
            Key::Z,
            Key::X,
            Key::C,
            Key::V,
            Key::B,
            Key::N,
            Key::M,
            Key::Comma_LessThan,
            Key::Period_GreaterThan,
            Key::Slash_Question,
            Key::Enter,
        ],
        [
            Key::None,
            Key::None,
            Key::Tab,
            Key::LeftAlt,
            Key::LeftGui,
            Key::Space,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
        ],
    ];
    const KEY_CODES_LOWER: [[Key; 12]; 4] = [
        [
            Key::Transparent,
            Key::Digit1_Exclamation,
            Key::Digit2_At,
            Key::Digit3_Number,
            Key::Digit4_Dollar,
            Key::Digit5_Percent,
            Key::Digit6_Circumflex,
            Key::Digit7_Ampersand,
            Key::Digit8_Asterisk,
            Key::Digit9_LeftParenthesis,
            Key::Digit0_RightParenthesis,
            Key::Tab,
        ],
        [
            Key::Transparent,
            Key::None,
            Key::None,
            Key::LeftParenthesis,
            Key::RightParenthesis,
            Key::None,
            Key::HyphenMinus_LowLine,
            Key::Equal_Plus,
            Key::LeftSquareBracket_LeftCurlyBracket,
            Key::RightSquareBracket_RightCurlyBracket,
            Key::VerticalBar,
            Key::Grave_Tilde,
        ],
        [
            Key::Transparent,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::LowLine,
            Key::Plus,
            Key::LeftCurlyBracket,
            Key::RightCurlyBracket,
            Key::Backslash_VerticalBar,
            Key::Tilde,
        ],
        [
            Key::None,
            Key::None,
            Key::None,
            Key::Transparent,
            Key::Transparent,
            Key::Transparent,
            Key::None,
            Key::Transparent,
            Key::Transparent,
            Key::Transparent,
            Key::None,
            Key::None,
        ],
    ];
    const KEY_CODES_RAISE: [[Key; 12]; 4] = [
        [
            Key::Transparent,
            Key::Exclamation,
            Key::At,
            Key::Hash,
            Key::Dollar,
            Key::Percent,
            Key::Circumflex,
            Key::Ampersand,
            Key::Asterisk,
            Key::LeftParenthesis,
            Key::RightParenthesis,
            Key::Transparent,
        ],
        [
            Key::Transparent,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::MediaVolumeDecrement,
            Key::MediaMute,
            Key::MediaVolumeIncrement,
            Key::None,
            Key::UpArrow,
            Key::None,
        ],
        [
            Key::Transparent,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::None,
            Key::MediaPrevTrack,
            Key::MediaPlayPause,
            Key::MediaNextTrack,
            Key::LeftArrow,
            Key::DownArrow,
            Key::RightArrow,
        ],
        [
            Key::None,
            Key::None,
            Key::Transparent,
            Key::Transparent,
            Key::Transparent,
            Key::Transparent,
            Key::None,
            Key::Transparent,
            Key::Transparent,
            Key::Transparent,
            Key::None,
            Key::None,
        ],
    ];
}

impl rustkbd_core::keyboard::Layout<2, Layer> for Layout {
    type Identifier = KeySwitchIdentifier;

    fn layer(&self, switches: &[Self::Identifier]) -> Layer {
        switches
            .iter()
            .map(|switch| match switch {
                KeySwitchIdentifier { row: 3, col: 7 } => Layer::Lower,
                KeySwitchIdentifier { row: 3, col: 8 } => Layer::Raise,
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

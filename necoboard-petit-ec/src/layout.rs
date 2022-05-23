use rustkbd_core::keyboard::{self, Key};

use crate::switch_identifier::KeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Layout {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

impl keyboard::Layer for Layer {
    fn below(&self) -> Option<Self> {
        let layers = [Self::Default, Self::Lower, Self::Raise];
        layers
            .iter()
            .enumerate()
            .find(|(_, l)| l == &self)
            .and_then(|(i, _)| if i > 0 { layers.get(i - 1) } else { None })
            .copied()
    }
}

impl Layout {
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

impl rustkbd_core::layout::Layout<3, Layer> for Layout {
    type Identifier = KeySwitchIdentifier;

    fn layer(&self, switches: &[Self::Identifier]) -> Layer {
        switches
            .iter()
            .map(|switch| match switch {
                KeySwitchIdentifier::Right { row: 1, col: 0 } => Layer::Lower,
                KeySwitchIdentifier::Right { row: 1, col: 1 } => Layer::Raise,
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

use rustkbd::keyboard::{self, layout, Key};

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
    const KEY_CODES_DEFAULT: [[Key; 4]; 4] = layout! {r"
        |  1  |  2  |  3  |  4  |
        |  5  |  6  |  7  |  8  |
        |  9  |  0  | Del |Enter|
        |     |     |     |     |
    "};
    const KEY_CODES_LOWER: [[Key; 4]; 4] = layout! {r"
        |  A  |  B  |  C  |  D  |
        |  E  |  F  |  G  |  H  |
        |  I  |  J  |  K  |  L  |
        |     |     |     |     |
    "};
    const KEY_CODES_RAISE: [[Key; 4]; 4] = layout! {r"
        |  M  |  N  |  O  |  P  |
        |  Q  |  R  |  S  |  T  |
        |  U  |  V  |  W  |  X  |
        |     |     |     |     |
    "};
}

impl rustkbd::keyboard::Layout<2> for Layout {
    type Identifier = KeySwitchIdentifier;
    type Layer = Layer;

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

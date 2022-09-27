use rustkbd::{
    keyboard::{self, layout, Key, Layout},
    split::SplitKeySwitchIdentifier,
};

use crate::key_matrix::KeySwitchIdentifier;

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SplitLayout {}

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

impl SplitLayout {
    const KEY_CODES_LEFT: [[Key; 2]; 2] = layout! {r"
        |  1  |  2  |
        | LSft| Del |
    "};
    const KEY_CODES_RIGHT: [[Key; 2]; 2] = layout! {r"
        |  3  |  4  |
        |     |     |
    "};

    const KEY_CODES_LOWER_LEFT: [[Key; 2]; 2] = layout! {r"
        |  A  |  B  |
        | Trn | Trn |
    "};
    const KEY_CODES_LOWER_RIGHT: [[Key; 2]; 2] = layout! {r"
        |  C  |  D  |
        |     |     |
    "};
    const KEY_CODES_RAISE_LEFT: [[Key; 2]; 2] = layout! {r"
        |     |MVlDn|
        |     |     |
    "};
    const KEY_CODES_RAISE_RIGHT: [[Key; 2]; 2] = layout! {r"
        |MPlPs|MVlUp|
        |     |     |
    "};
}

impl Layout<3> for SplitLayout {
    type Identifier = SplitKeySwitchIdentifier<2, KeySwitchIdentifier>;
    type Layer = Layer;

    fn layer(&self, switches: &[Self::Identifier]) -> Layer {
        switches
            .iter()
            .map(|switch| match switch {
                SplitKeySwitchIdentifier::Right(KeySwitchIdentifier { row: 1, col: 0 }) => {
                    Layer::Lower
                }
                SplitKeySwitchIdentifier::Right(KeySwitchIdentifier { row: 1, col: 1 }) => {
                    Layer::Raise
                }
                _ => Layer::Default,
            })
            .max()
            .unwrap_or_default()
    }

    fn key(&self, layer: Layer, switch: &Self::Identifier) -> Key {
        match (layer, *switch) {
            (Layer::Default, SplitKeySwitchIdentifier::Left(KeySwitchIdentifier { row, col })) => {
                Self::KEY_CODES_LEFT[row as usize][col as usize]
            }
            (Layer::Default, SplitKeySwitchIdentifier::Right(KeySwitchIdentifier { row, col })) => {
                Self::KEY_CODES_RIGHT[row as usize][col as usize]
            }
            (Layer::Lower, SplitKeySwitchIdentifier::Left(KeySwitchIdentifier { row, col })) => {
                Self::KEY_CODES_LOWER_LEFT[row as usize][col as usize]
            }
            (Layer::Lower, SplitKeySwitchIdentifier::Right(KeySwitchIdentifier { row, col })) => {
                Self::KEY_CODES_LOWER_RIGHT[row as usize][col as usize]
            }
            (Layer::Raise, SplitKeySwitchIdentifier::Left(KeySwitchIdentifier { row, col })) => {
                Self::KEY_CODES_RAISE_LEFT[row as usize][col as usize]
            }
            (Layer::Raise, SplitKeySwitchIdentifier::Right(KeySwitchIdentifier { row, col })) => {
                Self::KEY_CODES_RAISE_RIGHT[row as usize][col as usize]
            }
        }
    }
}

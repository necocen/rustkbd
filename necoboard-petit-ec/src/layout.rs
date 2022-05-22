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
    const KEYS: [[Key; 2]; 2] = [
        [Key::Digit1_Exclamation, Key::Digit2_At],
        [Key::LeftShift, Key::Delete],
    ];
}

impl rustkbd_core::layout::Layout<2, Layer> for Layout {
    type Identifier = KeySwitchIdentifier;

    fn layer(&self, _switches: &[Self::Identifier]) -> Layer {
        Layer::Default
    }

    fn key(&self, _layer: Layer, switch: Self::Identifier) -> Key {
        Self::KEYS[switch.row as usize][switch.col as usize]
    }
}

use rustkbd::keyboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeySwitchIdentifier {
    pub row: u8,
    pub col: u8,
}

impl From<[u8; 2]> for KeySwitchIdentifier {
    fn from(value: [u8; 2]) -> Self {
        KeySwitchIdentifier {
            row: value[0],
            col: value[1],
        }
    }
}

impl From<KeySwitchIdentifier> for [u8; 2] {
    fn from(value: KeySwitchIdentifier) -> Self {
        [value.row, value.col]
    }
}

impl keyboard::KeySwitchIdentifier<2> for KeySwitchIdentifier {}

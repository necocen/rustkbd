use rustkbd_core::keyboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySwitchIdentifier {
    Left { row: u8, col: u8 },
    Right { row: u8, col: u8 },
}

impl From<[u8; 3]> for KeySwitchIdentifier {
    fn from(value: [u8; 3]) -> Self {
        match (value[0], value[1], value[2]) {
            (0, row, col) => KeySwitchIdentifier::Left { row, col },
            (1, row, col) => KeySwitchIdentifier::Right { row, col },
            _ => panic!("unexpected switch data"), // TODO: TryFromにすべきか
        }
    }
}

impl From<KeySwitchIdentifier> for [u8; 3] {
    fn from(value: KeySwitchIdentifier) -> Self {
        match value {
            KeySwitchIdentifier::Left { row, col } => [0, row, col],
            KeySwitchIdentifier::Right { row, col } => [1, row, col],
        }
    }
}

impl keyboard::KeySwitchIdentifier<3> for KeySwitchIdentifier {}

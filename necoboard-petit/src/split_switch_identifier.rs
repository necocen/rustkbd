use rustkbd_core::keyboard::KeySwitchIdentifier;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitKeySwitchIdentifier {
    Left { row: u8, col: u8 },
    Right { row: u8, col: u8 },
}

impl From<[u8; 3]> for SplitKeySwitchIdentifier {
    fn from(value: [u8; 3]) -> Self {
        match (value[0], value[1], value[2]) {
            (0, row, col) => SplitKeySwitchIdentifier::Left { row, col },
            (1, row, col) => SplitKeySwitchIdentifier::Right { row, col },
            _ => panic!("unexpected switch data"), // TODO: TryFromにすべきか
        }
    }
}

impl From<SplitKeySwitchIdentifier> for [u8; 3] {
    fn from(value: SplitKeySwitchIdentifier) -> Self {
        match value {
            SplitKeySwitchIdentifier::Left { row, col } => [0, row, col],
            SplitKeySwitchIdentifier::Right { row, col } => [1, row, col],
        }
    }
}

impl KeySwitchIdentifier<3> for SplitKeySwitchIdentifier {}

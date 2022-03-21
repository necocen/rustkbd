use rustkbd_core::keyboard::KeySwitchIdentifier;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitKeySwitchIdentifier {
    Left(u8, u8),
    Right(u8, u8),
}

impl From<[u8; 3]> for SplitKeySwitchIdentifier {
    fn from(value: [u8; 3]) -> Self {
        match (value[0], value[1], value[2]) {
            (0, r, c) => SplitKeySwitchIdentifier::Left(r, c),
            (1, r, c) => SplitKeySwitchIdentifier::Right(r, c),
            _ => panic!("unexpected switch data"), // TODO: TryFromにすべきか
        }
    }
}

impl From<SplitKeySwitchIdentifier> for [u8; 3] {
    fn from(value: SplitKeySwitchIdentifier) -> Self {
        match value {
            SplitKeySwitchIdentifier::Left(r, c) => [0, r, c],
            SplitKeySwitchIdentifier::Right(r, c) => [1, r, c],
        }
    }
}

impl KeySwitchIdentifier<3> for SplitKeySwitchIdentifier {}

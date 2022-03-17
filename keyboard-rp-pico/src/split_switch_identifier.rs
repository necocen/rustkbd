use keyboard_core::key_switches::KeySwitchIdentifier;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitKeySwitchIdentifier {
    Left(u8, u8),
    Right(u8, u8),
}

impl From<[u8; 3]> for SplitKeySwitchIdentifier {
    fn from(value: [u8; 3]) -> Self {
        match (value[0], value[1], value[2]) {
            (0, c, r) => SplitKeySwitchIdentifier::Left(c, r),
            (1, c, r) => SplitKeySwitchIdentifier::Right(c, r),
            _ => panic!("unexpected switch data"), // TODO: TryFromにすべきか
        }
    }
}

impl From<SplitKeySwitchIdentifier> for [u8; 3] {
    fn from(value: SplitKeySwitchIdentifier) -> Self {
        match value {
            SplitKeySwitchIdentifier::Left(c, r) => [0, c, r],
            SplitKeySwitchIdentifier::Right(c, r) => [1, c, r],
        }
    }
}

impl KeySwitchIdentifier<3> for SplitKeySwitchIdentifier {}

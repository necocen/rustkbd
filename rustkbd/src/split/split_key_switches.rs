use embedded_hal::timer::CountDown;
use heapless::Vec;

use crate::{
    keyboard::{KeySwitchIdentifier, KeySwitches},
    split::{Connection, SplitCommunicator, SplitState},
};

pub struct SplitKeySwitches<
    const SZ: usize,
    const RO: usize,
    C: Connection,
    K: KeySwitches<SZ, RO>,
    T: CountDown,
> where
    T::Time: Copy,
{
    communicator: SplitCommunicator<SZ, RO, K, C, T>,
    switches: Vec<K::Identifier, RO>,
    underlying_switches: K,
    is_left: bool,
}

impl<const SZ: usize, const RO: usize, C: Connection, K: KeySwitches<SZ, RO>, T: CountDown>
    SplitKeySwitches<SZ, RO, C, K, T>
where
    T::Time: Copy,
{
    pub fn new(key_switches: K, connection: C, timer: T, timeout: T::Time, is_left: bool) -> Self {
        SplitKeySwitches {
            communicator: SplitCommunicator::new(connection, timer, timeout),
            switches: Vec::new(),
            underlying_switches: key_switches,
            is_left,
        }
    }

    pub fn poll(&mut self) {
        self.communicator.respond(&self.switches);
    }

    pub fn state(&self) -> SplitState {
        self.communicator.state()
    }

    fn establish(&mut self) {
        if let Err(e) = self.communicator.establish() {
            defmt::warn!("Failed to establish split connection: {}", e);
        }
    }

    fn _scan(&mut self) -> Vec<SplitKeySwitchIdentifier<SZ, K::Identifier>, RO> {
        if self.is_left && self.communicator.state() == SplitState::Undetermined {
            self.establish();
        }
        self.switches = self.underlying_switches.scan();
        let near_side = self.switches.clone();
        let far_side = self.communicator.request(&near_side);

        let left_side_transform: fn(K::Identifier) -> SplitKeySwitchIdentifier<SZ, K::Identifier> =
            SplitKeySwitchIdentifier::<SZ, K::Identifier>::Left;
        let right_side_transform: fn(K::Identifier) -> SplitKeySwitchIdentifier<SZ, K::Identifier> =
            SplitKeySwitchIdentifier::<SZ, K::Identifier>::Right;

        let (near_side_transform, far_side_transform) = if self.is_left {
            (left_side_transform, right_side_transform)
        } else {
            (right_side_transform, left_side_transform)
        };

        near_side
            .iter()
            .cloned()
            .map(near_side_transform)
            .chain(far_side.iter().cloned().map(far_side_transform))
            .take(RO)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitKeySwitchIdentifier<const SZ: usize, I: KeySwitchIdentifier<SZ>> {
    Left(I),
    Right(I),
}

macro_rules! impl_split_key_switches {
    ( $x:expr ) => {
        impl<I: KeySwitchIdentifier<$x>> From<[u8; $x + 1]> for SplitKeySwitchIdentifier<$x, I> {
            fn from(value: [u8; $x + 1]) -> Self {
                match value.split_first().unwrap() {
                    (&0, v) => {
                        SplitKeySwitchIdentifier::Left(I::from(<[u8; $x]>::try_from(v).unwrap()))
                    }
                    (&1, v) => {
                        SplitKeySwitchIdentifier::Right(I::from(<[u8; $x]>::try_from(v).unwrap()))
                    }
                    _ => panic!("unexpected switch data"), // TODO: TryFromにすべきか
                }
            }
        }
        impl<I: KeySwitchIdentifier<$x>> From<SplitKeySwitchIdentifier<$x, I>> for [u8; $x + 1] {
            fn from(value: SplitKeySwitchIdentifier<$x, I>) -> Self {
                let mut r = [0u8; $x + 1];
                let (h, t) = r.split_first_mut().unwrap();
                match value {
                    SplitKeySwitchIdentifier::Left(v) => {
                        *h = 0;
                        t.copy_from_slice(&v.into());
                    }
                    SplitKeySwitchIdentifier::Right(v) => {
                        *h = 1;
                        t.copy_from_slice(&v.into());
                    }
                }
                r
            }
        }
        impl<I: KeySwitchIdentifier<$x>> KeySwitchIdentifier<{ $x + 1 }>
            for SplitKeySwitchIdentifier<$x, I>
        {
        }
        impl<const RO: usize, C: Connection, K: KeySwitches<$x, RO>, T: CountDown>
            KeySwitches<{ $x + 1 }, RO> for SplitKeySwitches<$x, RO, C, K, T>
        where
            T::Time: Copy,
        {
            type Identifier = SplitKeySwitchIdentifier<$x, K::Identifier>;
            fn scan(&mut self) -> Vec<Self::Identifier, RO> {
                self._scan()
            }
        }
    };
}

impl_split_key_switches!(1);
impl_split_key_switches!(2);
impl_split_key_switches!(3);
impl_split_key_switches!(4);
impl_split_key_switches!(5);
impl_split_key_switches!(6);
impl_split_key_switches!(7);

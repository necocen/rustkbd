use core::cell::RefCell;

use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
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
    T: CountDown<Time = Microseconds<u64>>,
> {
    communicator: RefCell<SplitCommunicator<SZ, RO, K, C, T>>,
    switches: RefCell<Vec<K::Identifier, RO>>,
    underlying_switches: K,
    is_left: bool,
}

impl<
        const SZ: usize,
        const RO: usize,
        C: Connection,
        K: KeySwitches<SZ, RO>,
        T: CountDown<Time = Microseconds<u64>>,
    > SplitKeySwitches<SZ, RO, C, K, T>
{
    pub fn new(key_switches: K, connection: C, timer: T, is_left: bool) -> Self {
        SplitKeySwitches {
            communicator: RefCell::new(SplitCommunicator::new(connection, timer)),
            switches: RefCell::new(Vec::new()),
            underlying_switches: key_switches,
            is_left,
        }
    }

    pub fn poll(&self) {
        self.communicator
            .borrow_mut()
            .respond(&self.switches.borrow());
    }

    pub fn state(&self) -> SplitState {
        self.communicator.borrow().state()
    }

    fn establish(&self) {
        if let Err(e) = self.communicator.borrow_mut().establish() {
            defmt::warn!("Failed to establish split connection: {}", e);
        }
    }

    fn _scan(&self) -> Vec<SplitKeySwitchIdentifier<SZ, K::Identifier>, RO> {
        if self.is_left && self.communicator.borrow().state() == SplitState::Undetermined {
            self.establish();
        }
        *self.switches.borrow_mut() = self.underlying_switches.scan();
        let near_side = self.switches.borrow();
        let far_side = self.communicator.borrow_mut().request(&near_side);

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
        impl<
                const RO: usize,
                C: Connection,
                K: KeySwitches<$x, RO>,
                T: CountDown<Time = Microseconds<u64>>,
            > KeySwitches<{ $x + 1 }, RO> for SplitKeySwitches<$x, RO, C, K, T>
        {
            type Identifier = SplitKeySwitchIdentifier<$x, K::Identifier>;
            fn scan(&self) -> Vec<Self::Identifier, RO> {
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

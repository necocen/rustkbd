mod device_info;
mod hid_report;
mod key;
mod key_switches;
mod keyboard_state;
mod layer;
mod usb_communicator;

use core::cell::RefCell;

use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
use heapless::FnvIndexMap;
use usb_device::{
    class_prelude::{UsbBus, UsbBusAllocator},
    prelude::UsbDeviceState,
    UsbError,
};

use crate::{
    layout::Layout,
    split::{Connection, DummyConnection, SplitCommunicator, SplitState},
    Vec,
};

pub use device_info::DeviceInfo;
pub use key::Key;
pub use key_switches::{KeySwitchIdentifier, KeySwitches};
pub use keyboard_state::KeyboardState;
pub use layer::KeyboardLayer;

use self::usb_communicator::UsbCommunicator;

/// 最終的に送信されるキーのロールオーバー数。USBなので6。
pub(crate) const NUM_ROLLOVER: usize = 6;
/// キースイッチレベルでのロールオーバー数。modifier keysを含めるので6より大きい。
pub(crate) const NUM_SWITCH_ROLLOVER: usize = 12;

pub struct Keyboard<
    'b,
    const SZ: usize,
    B: UsbBus,
    K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
    S: Connection,
    T: CountDown<Time = Microseconds<u64>>,
    Y: KeyboardLayer,
    L: Layout<SZ, Y, Identifier = K::Identifier>,
> {
    usb_communicator: RefCell<UsbCommunicator<'b, B>>,
    split_communicator: RefCell<SplitCommunicator<SZ, K, S, T>>,
    key_switches: K,
    switches: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    layer: RefCell<Y>,
    layout: L,
    keys: RefCell<Vec<Key, NUM_ROLLOVER>>,
    pressed_switches: RefCell<FnvIndexMap<K::Identifier, Y, 16>>,
}

impl<
        'b,
        const SZ: usize,
        B: UsbBus,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        T: CountDown<Time = Microseconds<u64>>,
        Y: KeyboardLayer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, DummyConnection, T, Y, L>
{
    pub fn new(
        usb_bus_alloc: &'b UsbBusAllocator<B>,
        device_info: DeviceInfo,
        key_switches: K,
        timer: T,
        layout: L,
    ) -> Self {
        Keyboard {
            usb_communicator: RefCell::new(UsbCommunicator::new(device_info, usb_bus_alloc)),
            split_communicator: RefCell::new(SplitCommunicator::new(
                DummyConnection::default(),
                timer,
            )),
            key_switches,
            switches: RefCell::new(Vec::new()),
            layer: RefCell::new(Y::default()),
            layout,
            keys: RefCell::new(Vec::new()),
            pressed_switches: RefCell::new(FnvIndexMap::new()),
        }
    }
}

impl<
        'b,
        const SZ: usize,
        B: UsbBus,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        S: Connection,
        T: CountDown<Time = Microseconds<u64>>,
        Y: KeyboardLayer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, S, T, Y, L>
{
    pub fn new_split(
        usb_bus_alloc: &'b UsbBusAllocator<B>,
        device_info: DeviceInfo,
        key_switches: K,
        split_connection: S,
        timer: T,
        layout: L,
    ) -> Self {
        Keyboard {
            usb_communicator: RefCell::new(UsbCommunicator::new(device_info, usb_bus_alloc)),
            split_communicator: RefCell::new(SplitCommunicator::new(split_connection, timer)),
            key_switches,
            switches: RefCell::new(Vec::new()),
            layer: RefCell::new(Y::default()),
            layout,
            keys: RefCell::new(Vec::new()),
            pressed_switches: RefCell::new(FnvIndexMap::new()),
        }
    }

    pub fn get_state(&self) -> KeyboardState<Y, NUM_ROLLOVER> {
        let layer = *self.layer.borrow();
        let keys = self.keys.borrow().clone();
        let split = self.split_communicator.borrow().state();
        KeyboardState { layer, keys, split }
    }

    fn try_establish_split_connection_if_needed(&self) {
        if self.split_communicator.borrow().state() != SplitState::Undetermined {
            return;
        }
        if self.usb_communicator.borrow().state() != UsbDeviceState::Configured {
            return;
        }
        if let Err(e) = self.split_communicator.borrow_mut().establish() {
            defmt::warn!("Failed to establish split connection: {}", e);
        }
    }

    fn scan_switches(&self) -> Vec<K::Identifier, NUM_SWITCH_ROLLOVER> {
        *self.switches.borrow_mut() = self.key_switches.scan();

        let near_side = self.switches.borrow();
        let far_side = self.split_communicator.borrow_mut().request(&near_side);

        near_side
            .iter()
            .chain(far_side.iter())
            .take(NUM_SWITCH_ROLLOVER)
            .copied()
            .collect()
    }

    pub fn main_loop(&self) {
        self.try_establish_split_connection_if_needed();

        let switches = self.scan_switches();

        // グローバルなレイヤの決定
        let global_layer = self.layout.layer(&switches);

        // 個別のスイッチのレイヤの決定
        let switches_and_layers =
            determine_layers(&self.pressed_switches.borrow(), &switches, global_layer);

        // キーの決定
        let keys = determine_keys(&self.layout, &switches_and_layers);

        if !keys.is_empty() {
            defmt::debug!("{}", keys.as_slice());
        }

        // スイッチ押下状態の更新
        self.save_pressed_switches(&switches_and_layers);
        *self.layer.borrow_mut() = global_layer;
        *self.keys.borrow_mut() = keys;
    }

    pub fn usb_poll(&self) {
        self.usb_communicator.borrow_mut().poll()
    }

    pub fn split_poll(&self) {
        self.split_communicator
            .borrow_mut()
            .respond(&self.switches.borrow());
    }

    fn save_pressed_switches(&self, switches_and_layers: &[(&K::Identifier, Y)]) {
        *self.pressed_switches.borrow_mut() = switches_and_layers
            .iter()
            .cloned()
            .map(|(s, l)| (*s, l))
            .collect();
    }

    pub fn send_keys(&self) -> Result<(), UsbError> {
        if self.usb_communicator.borrow().state() != UsbDeviceState::Configured {
            return Ok(());
        }

        if self.split_communicator.borrow().state() == SplitState::Receiver
            || self.split_communicator.borrow().state() == SplitState::Undetermined
        {
            return Ok(());
        }

        self.usb_communicator
            .borrow()
            .send_keys(&self.keys.borrow())
    }
}

fn determine_layers<
    'a,
    Y: KeyboardLayer,
    SI: KeySwitchIdentifier<SZ>,
    const SZ: usize,
    const N: usize,
>(
    pressed_switches: &FnvIndexMap<SI, Y, N>,
    switches: &'a [SI],
    global_layer: Y,
) -> Vec<(&'a SI, Y), NUM_SWITCH_ROLLOVER> {
    // 個別のスイッチのレイヤの決定
    switches
        .iter()
        .map(|s| {
            let layer = if let Some(layer) = pressed_switches.get(s) {
                *layer
            } else {
                global_layer
            };
            (s, layer)
        })
        .collect::<Vec<(&SI, Y), NUM_SWITCH_ROLLOVER>>()
}

fn determine_keys<Y: KeyboardLayer, L: Layout<SZ, Y>, const SZ: usize>(
    layout: &L,
    switches_and_layers: &[(&L::Identifier, Y)],
) -> Vec<Key, NUM_ROLLOVER> {
    switches_and_layers
        .iter()
        .map(|(switch, mut layer)| {
            let mut key = layout.key(layer, switch);
            while key == Key::Transparent {
                if let Some(below) = layer.below() {
                    assert!(
                        layer != below,
                        "{}.below() does not change layer",
                        stringify!(Y)
                    );
                    layer = below;
                    key = layout.key(layer, switch);
                } else {
                    break;
                }
            }
            key
        })
        .filter(|key| !key.is_noop())
        .collect::<Vec<Key, NUM_ROLLOVER>>()
}

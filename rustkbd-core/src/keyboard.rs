mod device_info;
mod hid_report;
mod key;
mod key_switches;
mod keyboard_state;
mod layer;
mod usb_communicator;

use core::cell::RefCell;

use heapless::FnvIndexMap;
use usb_device::{class_prelude::UsbBus, prelude::UsbDeviceState, UsbError};

use crate::{layout::Layout, split::SplitState, Vec};

pub use device_info::DeviceInfo;
pub use key::Key;
pub use key_switches::{KeySwitchIdentifier, KeySwitches};
pub use keyboard_state::KeyboardState;
pub use layer::KeyboardLayer;
pub use usb_communicator::UsbCommunicator;

/// 最終的に送信されるキーのロールオーバー数。USBなので6。
pub(crate) const NUM_ROLLOVER: usize = 6;
/// キースイッチレベルでのロールオーバー数。modifier keysを含めるので6より大きい。
pub(crate) const NUM_SWITCH_ROLLOVER: usize = 12;

pub struct Keyboard<
    'b,
    const SZ: usize,
    B: UsbBus,
    K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
    Y: KeyboardLayer,
    L: Layout<SZ, Y, Identifier = K::Identifier>,
> {
    pub usb_communicator: RefCell<UsbCommunicator<'b, B>>,
    pub key_switches: K,
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
        Y: KeyboardLayer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, Y, L>
{
    pub fn new(usb_communicator: UsbCommunicator<'b, B>, key_switches: K, layout: L) -> Self {
        Keyboard {
            usb_communicator: RefCell::new(usb_communicator),
            key_switches,
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
        Y: KeyboardLayer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, Y, L>
{
    pub fn new_split(usb_communicator: UsbCommunicator<'b, B>, key_switches: K, layout: L) -> Self {
        Keyboard {
            usb_communicator: RefCell::new(usb_communicator),
            key_switches,
            layer: RefCell::new(Y::default()),
            layout,
            keys: RefCell::new(Vec::new()),
            pressed_switches: RefCell::new(FnvIndexMap::new()),
        }
    }

    pub fn get_state(&self) -> KeyboardState<Y, NUM_ROLLOVER> {
        let layer = *self.layer.borrow();
        let keys = self.keys.borrow().clone();
        KeyboardState {
            layer,
            keys,
            split: SplitState::NotAvailable,
        }
    }

    pub fn main_loop(&self) {
        let switches = self.key_switches.scan();

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
        *self.pressed_switches.borrow_mut() = switches_and_layers
            .into_iter()
            .map(|(s, l)| (*s, l))
            .collect();
        *self.layer.borrow_mut() = global_layer;
        *self.keys.borrow_mut() = keys;
    }

    pub fn usb_poll(&self) {
        self.usb_communicator.borrow_mut().poll()
    }

    pub fn send_keys(&self) -> Result<(), UsbError> {
        if self.usb_communicator.borrow().state() != UsbDeviceState::Configured {
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
        .collect()
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

mod external_communicator;
mod key;
mod key_switches;
mod keyboard_state;
mod layer;

use core::cell::RefCell;

use heapless::FnvIndexMap;

use crate::{layout::Layout, split::SplitState, Vec};

pub use external_communicator::ExternalCommunicator;
pub use key::Key;
pub use key_switches::{KeySwitchIdentifier, KeySwitches};
pub use keyboard_state::KeyboardState;
pub use layer::KeyboardLayer;

/// 最終的に送信されるキーのロールオーバー数。USBなので6。
const NUM_ROLLOVER: usize = 6;
/// キースイッチレベルでのロールオーバー数。modifier keysを含めるので6より大きい。
const NUM_SWITCH_ROLLOVER: usize = 12;

pub struct Keyboard<
    const SZ: usize,
    C: ExternalCommunicator,
    K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
    Y: KeyboardLayer,
    L: Layout<SZ, Y, Identifier = K::Identifier>,
> {
    pub communicator: C,
    pub key_switches: K,
    layer: RefCell<Y>,
    layout: L,
    keys: RefCell<Vec<Key, NUM_ROLLOVER>>,
    pressed_switches: RefCell<FnvIndexMap<K::Identifier, Y, 16>>,
}

impl<
        const SZ: usize,
        C: ExternalCommunicator,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        Y: KeyboardLayer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<SZ, C, K, Y, L>
{
    pub fn new(communicator: C, key_switches: K, layout: L) -> Self {
        Keyboard {
            communicator,
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

    pub fn send_keys(&self) -> Result<(), C::Error> {
        if !self.communicator.is_ready() {
            return Ok(());
        }

        self.communicator.send_keys(&self.keys.borrow())
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

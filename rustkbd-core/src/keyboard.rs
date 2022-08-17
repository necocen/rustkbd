mod device_info;
mod hid_report;
mod key;
mod key_switches;
mod keyboard_state;
mod layer;

use core::cell::RefCell;

use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
use heapless::FnvIndexMap;
use usb_device::{
    class_prelude::{UsbBus, UsbBusAllocator},
    device::{UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
    UsbError,
};
use usbd_hid::{
    descriptor::{MediaKeyboardReport, SerializedDescriptor},
    hid_class::HIDClass,
};

use crate::{
    layout::Layout,
    split::{Connection, ConnectionExt, DummyConnection, Error as SplitError, Message, SplitState},
    Vec,
};

use hid_report::HidKeyboardReport;

pub use device_info::DeviceInfo;
pub use key::Key;
pub use key_switches::{KeySwitchIdentifier, KeySwitches};
pub use keyboard_state::KeyboardState;
pub use layer::KeyboardLayer;

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
    usb_device: RefCell<UsbDevice<'b, B>>,
    keyboard_usb_hid: RefCell<HIDClass<'b, B>>,
    media_usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    split_connection: S,
    split_state: RefCell<SplitState>,
    switches: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    split_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    timer: RefCell<T>,
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
        let keyboard_usb_hid = HIDClass::new(usb_bus_alloc, HidKeyboardReport::desc(), 10);
        let media_usb_hid = HIDClass::new(usb_bus_alloc, MediaKeyboardReport::desc(), 10);
        let usb_device = UsbDeviceBuilder::new(
            usb_bus_alloc,
            UsbVidPid(device_info.vendor_id, device_info.product_id),
        )
        .manufacturer(device_info.manufacturer)
        .product(device_info.product_name)
        .serial_number(device_info.serial_number)
        .device_class(0)
        .build();
        Keyboard {
            keyboard_usb_hid: RefCell::new(keyboard_usb_hid),
            media_usb_hid: RefCell::new(media_usb_hid),
            usb_device: RefCell::new(usb_device),
            key_switches,
            split_connection: DummyConnection::default(),
            split_state: RefCell::new(SplitState::NotAvailable),
            switches: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
            timer: RefCell::new(timer),
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
        let keyboard_usb_hid = HIDClass::new(usb_bus_alloc, HidKeyboardReport::desc(), 10);
        let media_usb_hid = HIDClass::new(usb_bus_alloc, MediaKeyboardReport::desc(), 10);
        let usb_device = UsbDeviceBuilder::new(
            usb_bus_alloc,
            UsbVidPid(device_info.vendor_id, device_info.product_id),
        )
        .manufacturer(device_info.manufacturer)
        .product(device_info.product_name)
        .serial_number(device_info.serial_number)
        .device_class(0)
        .build();
        Keyboard {
            keyboard_usb_hid: RefCell::new(keyboard_usb_hid),
            media_usb_hid: RefCell::new(media_usb_hid),
            usb_device: RefCell::new(usb_device),
            key_switches,
            split_connection,
            split_state: RefCell::new(SplitState::Undetermined),
            switches: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
            timer: RefCell::new(timer),
            layer: RefCell::new(Y::default()),
            layout,
            keys: RefCell::new(Vec::new()),
            pressed_switches: RefCell::new(FnvIndexMap::new()),
        }
    }

    pub fn get_state(&self) -> KeyboardState<Y, NUM_ROLLOVER> {
        let layer = *self.layer.borrow();
        let keys = self.keys.borrow().clone();
        let split = *self.split_state.borrow();
        KeyboardState { layer, keys, split }
    }

    fn try_establish_split_connection_if_needed(&self) {
        if *self.split_state.borrow() != SplitState::Undetermined {
            return;
        }
        if self.usb_device.borrow().state() != UsbDeviceState::Configured {
            return;
        }
        if let Err(e) = self.split_establish() {
            defmt::warn!("Failed to establish split connection: {}", e);
        }
    }

    fn is_controller(&self) -> bool {
        *self.split_state.borrow() == SplitState::Controller
    }

    fn scan_switches(&self) -> Vec<K::Identifier, NUM_SWITCH_ROLLOVER> {
        *self.switches.borrow_mut() = self.key_switches.scan();

        if self.is_controller() {
            self.split_write_keys();
            if let Ok(Message::SwitchesReply(switches)) = self.split_read_message() {
                // replied
                *self.split_buf.borrow_mut() = switches;
            }
        }

        let near_side = self.switches.borrow();
        let far_side = self.split_buf.borrow();

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
        self.usb_device.borrow_mut().poll(&mut [
            &mut *self.keyboard_usb_hid.borrow_mut(),
            &mut *self.media_usb_hid.borrow_mut(),
        ]);
    }

    pub fn split_poll(&self) {
        match self.split_read_message() {
            Ok(Message::Switches(switches)) => {
                *self.split_buf.borrow_mut() = switches;
                self.split_write_keys_reply();
            }
            Ok(Message::SwitchesReply(switches)) => {
                // 通常ここには来ないがタイミングの問題で来る場合があるので適切にハンドリングする
                *self.split_buf.borrow_mut() = switches;
            }
            Ok(Message::FindReceiver) => {
                self.split_connection
                    .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::Acknowledge);
                *self.split_state.borrow_mut() = SplitState::Receiver;
            }
            _ => {}
        }
    }

    fn split_write_keys(&self) {
        let keys = self.switches.borrow().clone();
        self.split_connection.send_message(Message::Switches(keys));
    }

    fn split_write_keys_reply(&self) {
        let keys = self.switches.borrow().clone();
        self.split_connection
            .send_message(Message::SwitchesReply(keys));
    }

    fn split_establish(&self) -> Result<(), SplitError<S::Error>> {
        *self.split_state.borrow_mut() = SplitState::Undetermined;
        self.split_connection
            .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::FindReceiver);
        *self.split_state.borrow_mut() = match self.split_read_message()? {
            Message::Acknowledge => {
                defmt::info!("Split connection established");
                SplitState::Controller
            }
            _ => {
                defmt::warn!("Unexpected response");
                SplitState::Undetermined
            }
        };
        Ok(())
    }

    fn split_read_message(
        &self,
    ) -> Result<Message<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>, SplitError<S::Error>> {
        self.split_connection.read_message(
            &mut *self.timer.borrow_mut(),
            Microseconds::<u64>::new(10_000), // timeout in 10ms
        )
    }

    fn save_pressed_switches(&self, switches_and_layers: &[(&K::Identifier, Y)]) {
        *self.pressed_switches.borrow_mut() = switches_and_layers
            .iter()
            .cloned()
            .map(|(s, l)| (*s, l))
            .collect();
    }

    pub fn send_keys(&self) -> Result<(), UsbError> {
        if self.usb_device.borrow().state() != UsbDeviceState::Configured {
            return Ok(());
        }

        if *self.split_state.borrow() == SplitState::Receiver
            || *self.split_state.borrow() == SplitState::Undetermined
        {
            return Ok(());
        }

        let keys = self.keys.borrow();
        let report = keyboard_report(&keys);
        self.keyboard_usb_hid.borrow().push_input(&report)?;
        let media_key = keys.iter().find(|key| key.is_media_key());
        let report = media_report(media_key);
        self.media_usb_hid.borrow().push_input(&report)?;

        Ok(())
    }
}

fn keyboard_report(keys: &[Key]) -> HidKeyboardReport {
    let modifier = keys
        .iter()
        .map(|key| key.modifier_key_flag())
        .fold(0x00_u8, |acc, flg| acc | flg);
    let mut key_codes = [0u8; 6];
    keys.iter()
        .filter_map(|key| key.key_code())
        .take(NUM_ROLLOVER)
        .enumerate()
        .for_each(|(i, c)| key_codes[i] = c);
    HidKeyboardReport {
        modifier,
        reserved: 0,
        key_codes,
    }
}

fn media_report(key: Option<&Key>) -> MediaKeyboardReport {
    MediaKeyboardReport {
        usage_id: key.map(|key| key.media_usage_id()).unwrap_or(0),
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

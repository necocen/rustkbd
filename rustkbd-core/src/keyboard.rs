mod device_info;
mod hid_report;
mod key;
mod key_switches;
mod layer;
use core::cell::RefCell;

use defmt::Format;
use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
use heapless::Vec;
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
    split::{Connection, ConnectionExt, Error as SplitError, Message, SplitState},
};

use hid_report::HidKeyboardReport;

pub use device_info::DeviceInfo;
pub use key::Key;
pub use key_switches::{KeySwitchIdentifier, KeySwitches};
pub use layer::Layer;

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
    Y: Layer,
    L: Layout<SZ, Y, Identifier = K::Identifier>,
> {
    usb_device: RefCell<UsbDevice<'b, B>>,
    keyboard_usb_hid: RefCell<HIDClass<'b, B>>,
    media_usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    split_connection: S,
    split_state: RefCell<SplitState>,
    self_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    split_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    timer: RefCell<T>,
    layer: RefCell<Y>,
    layout: L,
    keys: RefCell<Vec<Key, NUM_ROLLOVER>>,
}

impl<
        'b,
        const SZ: usize,
        B: UsbBus,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        S: Connection,
        T: CountDown<Time = Microseconds<u64>>,
        Y: Layer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, S, T, Y, L>
{
    pub fn new(
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
            self_buf: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
            timer: RefCell::new(timer),
            layer: RefCell::new(Y::default()),
            layout,
            keys: RefCell::new(Vec::new()),
        }
    }

    pub fn main_loop(&self) {
        if self.is_split_undetermined()
            && self.usb_device.borrow().state() == UsbDeviceState::Configured
        {
            if let Err(e) = self.split_establish() {
                defmt::warn!("Split establish error: {}", e);
            }
        }

        // scan key matrix
        let scan = self.key_switches.scan();
        *self.self_buf.borrow_mut() = scan;

        if self.is_controller() {
            self.split_write_keys();
            if let Ok(Message::KeyInputReply(switches)) = self.split_read_message() {
                // replied
                *self.split_buf.borrow_mut() = switches;
            }
        }
        let self_side = self.self_buf.borrow();
        let other_side = self.split_buf.borrow();
        let switches = self_side
            .iter()
            .chain(other_side.iter())
            .take(NUM_ROLLOVER)
            .copied()
            .collect::<Vec<K::Identifier, NUM_ROLLOVER>>();
        *self.layer.borrow_mut() = self.layout.layer(&switches);
        let keys = self.keys_by_switches(&switches);
        if !keys.is_empty() {
            defmt::debug!("{}", keys.as_slice());
        }
        *self.keys.borrow_mut() = keys;

        if self.is_controller() {
            if let Err(e) = self.send_keys() {
                defmt::warn!("UsbError: {}", UsbErrorDisplay(e));
            }
        }
    }

    pub fn usb_poll(&self) {
        self.usb_device.borrow_mut().poll(&mut [
            &mut *self.keyboard_usb_hid.borrow_mut(),
            &mut *self.media_usb_hid.borrow_mut(),
        ]);
    }

    pub fn split_poll(&self) {
        match self.split_read_message() {
            Ok(Message::KeyInput(keys)) => {
                *self.split_buf.borrow_mut() = keys;
                self.split_write_keys_reply();
            }
            Ok(Message::KeyInputReply(keys)) => {
                // 通常ここには来ないがタイミングの問題で来る場合があるので適切にハンドリングする
                *self.split_buf.borrow_mut() = keys;
            }
            Ok(Message::FindReceiver) => {
                self.split_connection
                    .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::Acknowledge);
                *self.split_state.borrow_mut() = SplitState::Receiver;
            }
            _ => {}
        }
    }

    pub fn layer(&self) -> Y {
        *self.layer.borrow()
    }

    pub fn keys(&self) -> Vec<Key, NUM_ROLLOVER> {
        self.keys.borrow().clone()
    }

    pub fn split_state(&self) -> SplitState {
        *self.split_state.borrow()
    }

    fn split_write_keys(&self) {
        let keys = self.self_buf.borrow().clone();
        self.split_connection.send_message(Message::KeyInput(keys));
    }

    fn split_write_keys_reply(&self) {
        let keys = self.self_buf.borrow().clone();
        self.split_connection
            .send_message(Message::KeyInputReply(keys));
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

    fn is_controller(&self) -> bool {
        *self.split_state.borrow() == SplitState::Controller
    }

    fn is_split_undetermined(&self) -> bool {
        *self.split_state.borrow() == SplitState::Undetermined
    }

    fn keys_by_switches(&self, switches: &[K::Identifier]) -> Vec<Key, NUM_ROLLOVER> {
        let current_layer = *self.layer.borrow();
        switches
            .iter()
            .copied()
            .map(From::from)
            .map(|switch| {
                let mut layer = current_layer;
                let mut key = self.layout.key(layer, switch);
                while key == Key::Transparent {
                    if let Some(below) = layer.below() {
                        assert!(
                            layer != below,
                            "{}.below() does not change layer",
                            stringify!(Y)
                        );
                        layer = below;
                        key = self.layout.key(layer, switch);
                    } else {
                        break;
                    }
                }
                key
            })
            .filter(|key| !key.is_noop())
            .collect::<Vec<Key, NUM_ROLLOVER>>()
    }

    fn keyboard_report(&self, keys: &[Key]) -> HidKeyboardReport {
        let modifier = keys
            .iter()
            .map(|key| key.modifier_key_flag())
            .fold(0x00_u8, |acc, flg| acc | flg);
        let mut key_codes = [0u8; 6];
        keys.iter()
            .filter(|key| !key.is_modifier_key())
            .map(|key| *key as u8)
            .take(6)
            .enumerate()
            .for_each(|(i, c)| key_codes[i] = c);
        HidKeyboardReport {
            modifier,
            reserved: 0,
            key_codes,
        }
    }

    fn media_report(&self, key: Option<Key>) -> MediaKeyboardReport {
        MediaKeyboardReport {
            usage_id: key.map(|key| key.media_usage_id()).unwrap_or(0),
        }
    }

    fn send_keys(&self) -> Result<(), UsbError> {
        let keys = self.keys.borrow();
        let keyboard_keys = keys
            .iter()
            .filter(|key| key.is_keyboard_key())
            .cloned()
            .collect::<Vec<Key, NUM_ROLLOVER>>();
        let report = self.keyboard_report(&keyboard_keys);
        self.keyboard_usb_hid.borrow().push_input(&report)?;

        let media_key = keys.iter().find(|key| key.is_media_key()).cloned();
        let report = self.media_report(media_key);
        self.media_usb_hid.borrow().push_input(&report)?;
        Ok(())
    }
}

#[derive(Debug)]
struct UsbErrorDisplay(pub UsbError);

impl Format for UsbErrorDisplay {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", defmt::Debug2Format(&self.0));
    }
}

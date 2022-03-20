mod device_info;
mod hid_report;
mod key;
mod key_switches;
mod layer;
use core::cell::RefCell;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::Text,
    Drawable,
};
use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
use heapless::{String, Vec};
use usb_device::{
    class_prelude::{UsbBus, UsbBusAllocator},
    device::{UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_hid::{
    descriptor::{MediaKeyboardReport, SerializedDescriptor},
    hid_class::HIDClass,
};

use crate::{
    display::Display,
    layout::Layout,
    split::{Connection, ConnectionExt, Message, SplitState},
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
    D: Display<Color = BinaryColor>,
    S: Connection,
    T: CountDown<Time = Microseconds<u64>>,
    Y: Layer,
    L: Layout<SZ, Y, Identifier = K::Identifier>,
> {
    usb_device: RefCell<UsbDevice<'b, B>>,
    keyboard_usb_hid: RefCell<HIDClass<'b, B>>,
    media_usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    display: RefCell<D>,
    split_connection: S,
    split_state: RefCell<SplitState>,
    self_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    split_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    timer: RefCell<T>,
    layer: RefCell<Y>,
    layout: L,
}

impl<
        'b,
        const SZ: usize,
        B: UsbBus,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        D: Display<Color = BinaryColor>,
        S: Connection,
        T: CountDown<Time = Microseconds<u64>>,
        Y: Layer,
        L: Layout<SZ, Y, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, D, S, T, Y, L>
{
    pub fn new(
        usb_bus_alloc: &'b UsbBusAllocator<B>,
        device_info: DeviceInfo,
        key_switches: K,
        display: D,
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
            display: RefCell::new(display),
            split_connection,
            split_state: RefCell::new(SplitState::Undetermined),
            self_buf: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
            timer: RefCell::new(timer),
            layer: RefCell::new(Y::default()),
            layout,
        }
    }

    pub fn main_loop(&self) {
        // setup display
        let mut display = self.display.borrow_mut();
        let char_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        display.clear(BinaryColor::Off).ok();

        if self.is_split_undetermined()
            && self.usb_device.borrow().state() == UsbDeviceState::Configured
        {
            self.split_establish();
        }

        // scan key matrix
        let scan = self.key_switches.scan();
        *self.self_buf.borrow_mut() = scan;

        if self.is_controller() {
            self.split_write_keys();
            if let Some(Message::KeyInputReply(keys)) = self.split_read_message() {
                // replied
                *self.split_buf.borrow_mut() = keys;
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
        let keys = self.get_keys(&switches);

        if self.is_controller() {
            let keyboard_keys = keys
                .iter()
                .filter(|key| key.is_keyboard_key())
                .cloned()
                .collect::<Vec<Key, NUM_ROLLOVER>>();
            let report = self.keyboard_report(&keyboard_keys);
            self.keyboard_usb_hid.borrow().push_input(&report).ok();

            let media_key = keys.iter().find(|key| key.is_media_key()).cloned();
            let report = self.media_report(media_key);
            self.media_usb_hid.borrow().push_input(&report).ok();
        }

        // print pressed keys
        let mut string = String::<NUM_ROLLOVER>::new();
        keys.into_iter()
            .filter(|key| key.is_keyboard_key() && !key.is_modifier_key())
            .map(From::from)
            .for_each(|c| {
                string.push(c).ok();
            });
        Text::new(string.as_str(), Point::new(0, 10), char_style)
            .draw(&mut *display)
            .ok();

        // display "Receiver" or "Controller"
        let state = match *self.split_state.borrow() {
            SplitState::Undetermined => "Undetermined",
            SplitState::WaitingForReceiver => "Waiting",
            SplitState::Controller => "Controller",
            SplitState::Receiver => "Receiver",
        };
        Text::new(state, Point::new(0, 22), char_style)
            .draw(&mut *display)
            .ok();

        if D::REQUIRES_FLUSH {
            display.flush().ok();
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
            Some(Message::KeyInput(keys)) => {
                *self.split_buf.borrow_mut() = keys;
                self.split_write_keys_reply();
            }
            Some(Message::KeyInputReply(keys)) => {
                // 通常ここには来ないがタイミングの問題で来る場合があるので適切にハンドリングする
                *self.split_buf.borrow_mut() = keys;
            }
            Some(Message::FindReceiver) => {
                self.split_connection
                    .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::Acknowledge);
                *self.split_state.borrow_mut() = SplitState::Receiver;
            }
            _ => {}
        }
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

    fn split_establish(&self) {
        *self.split_state.borrow_mut() = SplitState::WaitingForReceiver;
        self.split_connection
            .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::FindReceiver);
        *self.split_state.borrow_mut() = match self.split_read_message() {
            Some(Message::Acknowledge) => SplitState::Controller,
            _ => SplitState::Undetermined,
        };
    }

    fn split_read_message(&self) -> Option<Message<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>> {
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

    fn get_keys(&self, switches: &[K::Identifier]) -> Vec<Key, NUM_ROLLOVER> {
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
}

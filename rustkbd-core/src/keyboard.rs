mod keyboard_handedness;
mod keyboard_report;
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
use usbd_hid::{descriptor::SerializedDescriptor, hid_class::HIDClass};

use crate::{
    display::KeyboardDisplay,
    key_switches::KeySwitches,
    layout::KeyLayout,
    split::{SplitConnection, SplitConnectionExt, SplitMessage, SplitState},
};

pub use keyboard_handedness::KeyboardHandedness;
use keyboard_report::KeyboardReport;

/// 最終的に送信されるキーのロールオーバー数。USBなので6。
pub(crate) const NUM_ROLLOVER: usize = 6;
/// キースイッチレベルでのロールオーバー数。modifier keysを含めるので6より大きい。
pub(crate) const NUM_SWITCH_ROLLOVER: usize = 12;

pub struct Keyboard<
    'b,
    const SZ: usize,
    B: UsbBus,
    K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
    D: KeyboardDisplay<Color = BinaryColor>,
    S: SplitConnection,
    T: CountDown<Time = Microseconds<u64>>,
    L: KeyLayout<SZ, NUM_ROLLOVER, Identifier = K::Identifier>,
> {
    usb_device: RefCell<UsbDevice<'b, B>>,
    usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    display: RefCell<D>,
    split_connection: S,
    split_state: RefCell<SplitState>,
    self_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    split_buf: RefCell<Vec<K::Identifier, NUM_SWITCH_ROLLOVER>>,
    timer: RefCell<T>,
    layout: L,
}

impl<
        'b,
        const SZ: usize,
        B: UsbBus,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        D: KeyboardDisplay<Color = BinaryColor>,
        S: SplitConnection,
        T: CountDown<Time = Microseconds<u64>>,
        L: KeyLayout<SZ, NUM_ROLLOVER, Identifier = K::Identifier>,
    > Keyboard<'b, SZ, B, K, D, S, T, L>
{
    pub fn new(
        usb_bus_alloc: &'b UsbBusAllocator<B>,
        key_switches: K,
        display: D,
        split_connection: S,
        timer: T,
        layout: L,
    ) -> Self {
        let usb_hid = HIDClass::new(usb_bus_alloc, KeyboardReport::desc(), 10);
        let usb_device = UsbDeviceBuilder::new(usb_bus_alloc, UsbVidPid(0xfeed, 0x802f))
            .manufacturer("necocen")
            .product("necoboard")
            .serial_number("17")
            .device_class(0)
            .build();
        Keyboard {
            usb_hid: RefCell::new(usb_hid),
            usb_device: RefCell::new(usb_device),
            key_switches,
            display: RefCell::new(display),
            split_connection,
            split_state: RefCell::new(SplitState::Undetermined),
            self_buf: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
            timer: RefCell::new(timer),
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
            if let Some(SplitMessage::KeyInputReply(keys)) = self.split_read_message() {
                // replied
                *self.split_buf.borrow_mut() = keys;
            }
        }
        let self_side = self.self_buf.borrow();
        let other_side = self.split_buf.borrow();
        let keys = self_side
            .iter()
            .chain(other_side.iter())
            .take(NUM_ROLLOVER)
            .copied()
            .map(From::from)
            .collect::<Vec<K::Identifier, NUM_ROLLOVER>>();
        let key_codes = self.layout.key_codes(&keys);
        if self.is_controller() {
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                key_codes,
            };
            self.usb_hid.borrow().push_input(&report).ok();
        }

        // print pressed keys
        let mut string = String::<NUM_ROLLOVER>::new();
        for key in key_codes.iter() {
            if *key != 0 {
                string.push((key - 0x1e + b'1') as char).ok();
            }
        }
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
        self.usb_device
            .borrow_mut()
            .poll(&mut [&mut *self.usb_hid.borrow_mut()]);
    }

    pub fn split_poll(&self) {
        match self.split_read_message() {
            Some(SplitMessage::KeyInput(keys)) => {
                *self.split_buf.borrow_mut() = keys;
                self.split_write_keys_reply();
            }
            Some(SplitMessage::KeyInputReply(keys)) => {
                // 通常ここには来ないがタイミングの問題で来る場合があるので適切にハンドリングする
                *self.split_buf.borrow_mut() = keys;
            }
            Some(SplitMessage::FindReceiver) => {
                self.split_connection.send_message(
                    SplitMessage::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::Acknowledge,
                );
                *self.split_state.borrow_mut() = SplitState::Receiver;
            }
            _ => {}
        }
    }

    fn split_write_keys(&self) {
        let keys = self.self_buf.borrow().clone();
        self.split_connection
            .send_message(SplitMessage::KeyInput(keys));
    }

    fn split_write_keys_reply(&self) {
        let keys = self.self_buf.borrow().clone();
        self.split_connection
            .send_message(SplitMessage::KeyInputReply(keys));
    }

    fn split_establish(&self) {
        *self.split_state.borrow_mut() = SplitState::WaitingForReceiver;
        self.split_connection
            .send_message(SplitMessage::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::FindReceiver);
        *self.split_state.borrow_mut() = match self.split_read_message() {
            Some(SplitMessage::Acknowledge) => SplitState::Controller,
            _ => SplitState::Undetermined,
        };
    }

    fn split_read_message(&self) -> Option<SplitMessage<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>> {
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
}

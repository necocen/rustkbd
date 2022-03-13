mod keyboard_handedness;
mod keyboard_report;
mod keyboard_state;
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

use crate::{display::KeyboardDisplay, key_switches::KeySwitches, split::SplitConnection};

pub use keyboard_handedness::KeyboardHandedness;
use keyboard_report::KeyboardReport;
use keyboard_state::KeyboardState;

pub struct Keyboard<
    'b,
    B: UsbBus,
    K: KeySwitches<Identifier = (u8, u8)>,
    D: KeyboardDisplay<Color = BinaryColor>,
    S: SplitConnection,
    T: CountDown<Time = Microseconds<u64>>,
> {
    usb_device: RefCell<UsbDevice<'b, B>>,
    usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    display: RefCell<D>,
    split_connection: S,
    handedness: KeyboardHandedness,
    split_state: RefCell<KeyboardState>,
    self_buf: RefCell<Vec<(u8, u8), 6>>,
    split_buf: RefCell<Vec<(u8, u8), 6>>,
    timer: RefCell<T>,
}

impl<
        'b,
        B: UsbBus,
        K: KeySwitches<Identifier = (u8, u8)>,
        D: KeyboardDisplay<Color = BinaryColor>,
        S: SplitConnection,
        T: CountDown<Time = Microseconds<u64>>,
    > Keyboard<'b, B, K, D, S, T>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];

    pub fn new(
        usb_bus_alloc: &'b UsbBusAllocator<B>,
        key_switches: K,
        display: D,
        split_connection: S,
        timer: T,
        handedness: KeyboardHandedness,
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
            handedness,
            split_state: RefCell::new(KeyboardState::Undetermined),
            self_buf: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
            timer: RefCell::new(timer),
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
            if self.split_read_head_with_timeout() == Some(0x01) {
                // reply
                self.split_read_keys();
            }
        }
        let self_side = self.self_buf.borrow();
        let other_side = self.split_buf.borrow();
        let key_codes = match self.handedness {
            KeyboardHandedness::NotApplicable | KeyboardHandedness::Left => {
                self.key_codes(&self_side, &other_side)
            }
            KeyboardHandedness::Right => self.key_codes(&other_side, &self_side),
        };
        if self.is_controller() {
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                key_codes,
            };
            self.usb_hid.borrow().push_input(&report).ok();
        }

        // print pressed keys
        let mut string = String::<6>::new();
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
            KeyboardState::Undetermined => "Undetermined",
            KeyboardState::WaitingForReceiver => "Waiting",
            KeyboardState::Controller => "Controller",
            KeyboardState::Receiver => "Receiver",
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
        let head = self.split_read_head_with_timeout();
        if head.is_none() {
            return;
        }
        let head = head.unwrap();
        match head {
            0x00 => {
                self.split_read_keys();
                self.split_write_keys_reply();
            }
            0x01 => {
                // 通常ここには来ないがタイミングの問題で来る場合があるので適切にハンドリングする
                self.split_read_keys();
            }
            0xff => {
                self.split_connection.write(&[0xfe]);
                *self.split_state.borrow_mut() = KeyboardState::Receiver;
            }
            _ => {}
        }
    }

    fn split_read_keys(&self) {
        let mut buf: [u8; 16] = [0; 16];
        self.split_connection.read(&mut buf[..1]);
        let len = buf[0] as usize;
        if len == 0 {
            *self.split_buf.borrow_mut() = Vec::new();
            return;
        }

        self.split_connection.read(&mut buf[..(len * 2)]);
        let mut split_buf = self.split_buf.borrow_mut();
        *split_buf = (0..len)
            .map(|x| x * 2)
            .map(|x| (buf[x], buf[x + 1]))
            .collect();
    }

    fn split_write_keys(&self) {
        let keys = self.self_buf.borrow();
        let len = keys.len() as u8;
        let data = core::iter::once(0x00)
            .chain(core::iter::once(len))
            .chain(keys.iter().flat_map(|(col, row)| [*col, *row]))
            .collect::<Vec<u8, 15>>();
        self.split_connection.write(&data);
    }

    fn split_write_keys_reply(&self) {
        let keys = self.self_buf.borrow();
        let len = keys.len() as u8;
        let data = core::iter::once(0x01)
            .chain(core::iter::once(len))
            .chain(keys.iter().flat_map(|(col, row)| [*col, *row]))
            .collect::<Vec<u8, 15>>();
        self.split_connection.write(&data);
    }

    fn split_establish(&self) {
        *self.split_state.borrow_mut() = KeyboardState::WaitingForReceiver;
        // とりあえず0xffをアレとする
        self.split_connection.write(&[0xff]);
        *self.split_state.borrow_mut() = match self.split_read_head_with_timeout() {
            Some(0xfe) => KeyboardState::Controller,
            _ => KeyboardState::Undetermined,
        };
    }

    fn split_read_head_with_timeout(&self) -> Option<u8> {
        let mut buf = [0u8; 1];
        let result = self.split_connection.read_with_timeout(
            &mut buf,
            &mut *self.timer.borrow_mut(),
            Microseconds::<u64>::new(10_000),
        );
        if result {
            Some(buf[0])
        } else {
            None
        }
    }

    fn is_controller(&self) -> bool {
        *self.split_state.borrow() == KeyboardState::Controller
    }

    fn is_split_undetermined(&self) -> bool {
        *self.split_state.borrow() == KeyboardState::Undetermined
    }

    fn key_codes(&self, left: &[(u8, u8)], right: &[(u8, u8)]) -> [u8; 6] {
        let mut keys = [0u8; 6];

        let left = left
            .iter()
            .map(|(col, row)| Self::KEY_CODES_LEFT[*col as usize][*row as usize]);
        let right = right
            .iter()
            .map(|(col, row)| Self::KEY_CODES_RIGHT[*col as usize][*row as usize]);

        for (i, key) in left.chain(right).take(6).enumerate() {
            keys[i] = key;
        }
        keys
    }
}

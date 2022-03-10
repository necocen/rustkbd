use core::cell::RefCell;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::Text,
    Drawable,
};
use heapless::{String, Vec};
use usb_device::{
    class_prelude::{UsbBus, UsbBusAllocator},
    device::{UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
};
use usbd_hid::{descriptor::SerializedDescriptor, hid_class::HIDClass};

use crate::{
    display::KeyboardDisplay, key_switches::KeySwitches, keyboard_report::KeyboardReport,
    split_connection::SplitConnection,
};

pub struct Keyboard<
    'b,
    B: UsbBus,
    K: KeySwitches<Identifier = (u8, u8)>,
    D: KeyboardDisplay<Color = BinaryColor>,
    S: SplitConnection,
> {
    usb_device: RefCell<UsbDevice<'b, B>>,
    usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    display: RefCell<D>,
    split_connection: S,
    is_left_hand: bool,
    self_buf: RefCell<Vec<(u8, u8), 6>>,
    split_buf: RefCell<Vec<(u8, u8), 6>>,
}

impl<
        'b,
        B: UsbBus,
        K: KeySwitches<Identifier = (u8, u8)>,
        D: KeyboardDisplay<Color = BinaryColor>,
        S: SplitConnection,
    > Keyboard<'b, B, K, D, S>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];

    pub fn new(
        usb_bus_alloc: &'b UsbBusAllocator<B>,
        key_switches: K,
        display: D,
        split_connection: S,
        is_left_hand: bool,
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
            is_left_hand,
            self_buf: RefCell::new(Vec::new()),
            split_buf: RefCell::new(Vec::new()),
        }
    }

    pub fn main_loop(&self) {
        // scan key matrix
        let scan = self.key_switches.scan();
        *self.self_buf.borrow_mut() = scan;
        if self.is_controller() {
            self.split_write();
            self.split_read();
        }
        let self_side = self.self_buf.borrow();
        let other_side = self.split_buf.borrow();
        let key_codes = if self.is_left_hand {
            self.key_codes(&self_side, &other_side)
        } else {
            self.key_codes(&other_side, &self_side)
        };

        if self.is_controller() {
            let report = KeyboardReport {
                modifier: 0,
                reserved: 0,
                key_codes,
            };
            self.usb_hid.borrow().push_input(&report).ok();
        }

        // setup display
        let mut display = self.display.borrow_mut();
        display.clear(BinaryColor::Off).ok();

        // print pressed keys
        let mut string = String::<6>::new();
        for key in key_codes.iter() {
            if *key != 0 {
                string.push((key - 0x1e + b'1') as char).ok();
            }
        }
        let char_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
        Text::new(string.as_str(), Point::new(0, 10), char_style)
            .draw(&mut *display)
            .ok();

        // display "Receiver" or "Controller"
        if self.is_receiver() {
            Text::new("Receiver", Point::new(0, 22), char_style)
                .draw(&mut *display)
                .ok();
        } else {
            Text::new("Controller", Point::new(0, 22), char_style)
                .draw(&mut *display)
                .ok();
        }

        if D::REQUIRES_FLUSH {
            display.flush().ok(); // かなりここ律速
        }
    }

    pub fn usb_poll(&self) {
        self.usb_device
            .borrow_mut()
            .poll(&mut [&mut *self.usb_hid.borrow_mut()]);
    }

    pub fn split_poll(&self) {
        if self.is_controller() {
            return;
        }
        self.split_read();
        self.split_write();
    }

    fn split_read(&self) {
        let mut buf: [u8; 16] = [0; 16];
        self.split_connection.read(&mut buf[..1]);
        if buf[0] == 0 {
            return;
        }
        let len = (buf[0] as usize) - 1;
        self.split_connection.read(&mut buf[..len]);

        let mut split_buf = self.split_buf.borrow_mut();
        *split_buf = (0..(len / 2))
            .map(|x| x * 2)
            .map(|x| (buf[x], buf[x + 1]))
            .collect();
    }

    fn split_write(&self) {
        let keys = self.self_buf.borrow();
        let len = (keys.len() * 2 + 1) as u8;
        let data = core::iter::once(len)
            .chain(keys.iter().flat_map(|(col, row)| [*col, *row]))
            .collect::<Vec<u8, 15>>();
        self.split_connection.write(&data);
    }

    fn is_controller(&self) -> bool {
        !self.is_receiver()
    }

    fn is_receiver(&self) -> bool {
        self.usb_device.borrow().state() != UsbDeviceState::Configured
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

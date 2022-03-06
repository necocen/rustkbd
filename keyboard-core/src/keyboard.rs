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

use crate::{display::KeyboardDisplay, key_switches::KeySwitches, keyboard_report::KeyboardReport};

pub struct Keyboard<
    'b,
    B: UsbBus,
    K: KeySwitches<Identifier = (u8, u8)>,
    D: KeyboardDisplay<Color = BinaryColor>,
> {
    usb_device: RefCell<UsbDevice<'b, B>>,
    usb_hid: RefCell<HIDClass<'b, B>>,
    key_switches: K,
    display: RefCell<D>,
    is_recv: bool,
}

impl<
        'b,
        B: UsbBus,
        K: KeySwitches<Identifier = (u8, u8)>,
        D: KeyboardDisplay<Color = BinaryColor>,
    > Keyboard<'b, B, K, D>
{
    const KEY_CODES_LEFT: [[u8; 2]; 2] = [[0x1e, 0x1f], [0x20, 0x21]];
    const KEY_CODES_RIGHT: [[u8; 2]; 2] = [[0x22, 0x23], [0x24, 0x25]];

    pub fn new(usb_bus_alloc: &'b UsbBusAllocator<B>, key_switches: K, display: D) -> Self {
        let usb_hid = HIDClass::new(usb_bus_alloc, KeyboardReport::desc(), 10);
        let usb_device = UsbDeviceBuilder::new(usb_bus_alloc, UsbVidPid(0xfeed, 0x802f))
            .manufacturer("necocen")
            .product("necoboard")
            .serial_number("17")
            .device_class(0)
            .build();

        // TODO: ここでdelayしたい
        //let is_recv = usb_device.state() == UsbDeviceState::Default;
        let is_recv = false;
        Keyboard {
            usb_hid: RefCell::new(usb_hid),
            usb_device: RefCell::new(usb_device),
            key_switches,
            display: RefCell::new(display),
            is_recv,
        }
    }

    pub fn main_loop(&self) {
        //let im = Image::new(&Self::RUST_LOGO, Point::new(0, 0));
        //self.usart_controller.put(&left);
        //let right = self.usart_controller.get();
        //im.draw(&mut *display).ok();

        // scan key matrix
        let scan = self.key_switches.scan();
        let key_codes = if self.is_recv {
            self.key_codes(Vec::new(), scan)
        } else {
            self.key_codes(scan, Vec::new())
        };

        if !self.is_recv {
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
        if self.is_recv {
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

    fn key_codes(&self, left: Vec<(u8, u8), 6>, right: Vec<(u8, u8), 6>) -> [u8; 6] {
        let mut keys = [0u8; 6];

        let left = left
            .into_iter()
            .map(|(col, row)| Self::KEY_CODES_LEFT[col as usize][row as usize]);
        let right = right
            .into_iter()
            .map(|(col, row)| Self::KEY_CODES_RIGHT[col as usize][row as usize]);

        for (i, key) in left.chain(right).take(6).enumerate() {
            keys[i] = key;
        }
        keys
    }
}

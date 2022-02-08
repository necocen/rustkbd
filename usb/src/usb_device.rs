use core::{
    cell::RefCell,
    intrinsics::copy_nonoverlapping,
    sync::atomic::{AtomicU8, Ordering},
};

use atmega_hal::pac::{PLL, USB_DEVICE};
use avr_device::interrupt::{self, Mutex};
use avr_progmem::progmem;
use descriptors::{
    ConfigDescriptor, DeviceDescriptor, HidDescriptor, HidFunction, HidReport, InterfaceDescriptor,
    USBConfiguration,
};
pub use device_state::DeviceState;
pub use setup_packet::SetupPacket;

use self::{
    descriptors::EndpointDescriptor,
    request_type::{Direction, Recipient, Type},
};

mod descriptors;
mod device_state;
mod request_type;
mod setup_packet;

static MY_USB: Mutex<RefCell<Option<USB_DEVICE>>> = Mutex::new(RefCell::new(None));
static MY_PLL: Mutex<RefCell<Option<PLL>>> = Mutex::new(RefCell::new(None));
static DEVICE_STATUS: AtomicU8 = AtomicU8::new(DeviceState::Unattached as u8);
static KEYBOARD_PROTOCOL: AtomicU8 = AtomicU8::new(0);
static KEYBOARD_IDLE_VALUE: AtomicU8 = AtomicU8::new(125);

#[derive(Debug)]
#[non_exhaustive]
pub struct UsbDevice {}

impl UsbDevice {
    pub fn new(usb: USB_DEVICE, pll: PLL) -> Self {
        interrupt::free(|cs| {
            usb.usbcon.reset();
            usb.uhwcon.reset();
            usb.udcon.reset();
            usb.udien.reset();
            // 一瞬切った方がいいらしい cf. https://kampi.gitbook.io/avr/lets-use-usb/initialize-the-usb-as-a-usb-device
            usb.usbcon.modify(|_, w| w.usbe().clear_bit());
            usb.usbcon.modify(|_, w| w.usbe().set_bit());
            // After a reset, the USB device mode is automatically activated because the external UID pin of the microcontroller is deactivated (UIDE = 0) and the UIMOD bit is set. Furthermore, the PLL for the USB module gets turned off during a reset by setting the FRZCLK bit in the USBCON register. This bit must also be deleted after a reset.
            usb.usbcon.modify(|_, w| w.frzclk().clear_bit());
            // It is recommended to use the internal voltage regulator to ensure the supply voltage of the data lines.
            usb.uhwcon.modify(|_, w| w.uvrege().set_bit());
            // In order for the microcontroller to attach to the USB and the host to recognize the new device, the VBUS pad must also be activated via the OTGPADE bit in the USBCON register.
            usb.usbcon.modify(|_, w| w.otgpade().set_bit());
            // low speed mode (full speed modeのときは外部発振子が必要らしい？)←Pro Microなので載ってると思われる。が開発中はデバッグしやすいようにlsmでやる
            usb.udcon.modify(|_, w| w.lsm().set_bit());
            // Only the VBUS interrupt is currently required, as this is used for plug-in detection. This interrupt is activated via the VBUSTE bit in the USBCON register.
            usb.usbcon.modify(|_, w| w.vbuste().set_bit());
            // The DETACH bit in the UDCON register must be cleared so  the selected pull-up resistor is connected to the corresponding data line and the device is detected by the host.
            usb.udcon.modify(|_, w| w.detach().clear_bit());
            // The end of the USB reset can be detected using the EORSTI bit in the UDINT register.
            usb.udien.modify(|_, w| w.eorste().set_bit());
            MY_USB.borrow(cs).replace(Some(usb));
            MY_PLL.borrow(cs).replace(Some(pll));
        });
        UsbDevice {}
    }

    pub fn get_status(&self) -> DeviceState {
        get_device_status()
    }

    pub fn send(&self, data: [u8; 8]) {
        if self.get_status() != DeviceState::Configured {
            return;
        }
        interrupt::free(|cs| {
            let usb = MY_USB.borrow(cs).borrow();
            let usb = usb.as_ref().unwrap();
            let current_ep = usb.uenum.read().bits();
            unsafe { usb.uenum.write(|w| w.bits(3)) }; // KEYBOARD_ENDPOINT_NUM
            while usb.ueintx.read().rwal().bit_is_clear() {}

            unsafe {
                for byte in data.iter() {
                    usb.uedatx.write(|w| w.bits(*byte));
                }
            }
            usb.ueintx.modify(|_, w| w.fifocon().clear_bit());
            unsafe { usb.uenum.write(|w| w.bits(current_ep)) };
        });
    }
}

#[avr_device::interrupt(atmega32u4)]
fn USB_GEN() {
    // The PLL is configured in the ISR of the USB controller as soon as a VBUS interrupt has been triggered.
    interrupt::free(|cs| {
        let usb = MY_USB.borrow(cs).borrow();
        let usb = usb.as_ref().unwrap();
        let pll = MY_PLL.borrow(cs).borrow();
        let pll = pll.as_ref().unwrap();
        if usb.usbint.read().vbusti().bit_is_set() {
            usb.usbint.modify(|_, w| w.vbusti().clear_bit());
            if usb.usbsta.read().vbus().bit_is_set() {
                pll.pllcsr
                    .modify(|_, w| w.pindiv().set_bit().plle().set_bit()); // 16MHz
                while pll.pllcsr.read().plock().bit_is_clear() {}
                DEVICE_STATUS.store(DeviceState::Powered as u8, Ordering::Relaxed);
            } else {
                pll.pllcsr.modify(|_, w| unsafe { w.bits(0) });
                DEVICE_STATUS.store(DeviceState::Unattached as u8, Ordering::Relaxed);
            }
        }
        if usb.udint.read().eorsti().bit_is_set() {
            // end of reset interrupt
            unsafe { usb.udint.write(|w| w.bits(0)) };
            if configure_endpoint(usb, 0, 8, 0, false) {
                unsafe { usb.uerst.write(|w| w.bits(1)) };
                unsafe { usb.uerst.write(|w| w.bits(0)) };

                // re-enable receive setup packet interrupt
                usb.ueienx.write(|w| w.rxstpe().set_bit());
                DEVICE_STATUS.store(DeviceState::Reset as u8, Ordering::Relaxed);
            }
        }
    });
}

#[avr_device::interrupt(atmega32u4)]
fn USB_COM() {
    interrupt::free(|cs| {
        if get_device_status() == DeviceState::Unattached {
            return;
        }
        let usb = MY_USB.borrow(cs).borrow();
        let usb = usb.as_ref().unwrap();
        let current_endpoint = usb.uenum.read().bits();
        unsafe { usb.uenum.write(|w| w.bits(0)) }; // 0番エンドポイントを操作
        if usb.ueintx.read().rxstpi().bit_is_set() {
            // setup
            usb_control_request(usb);
        }
        unsafe { usb.uenum.write(|w| w.bits(current_endpoint)) }; // 通らない？
    });
}

fn get_device_status() -> DeviceState {
    unsafe { core::mem::transmute(DEVICE_STATUS.load(Ordering::Relaxed)) }
}

fn configure_endpoint(usb: &USB_DEVICE, ep: u8, size: u8, ep_type: u8, double_bank: bool) -> bool {
    let addr_tmp = ep & 0x0F;
    for i in addr_tmp..0x07 {
        unsafe { usb.uenum.write(|w| w.bits(i)) };

        if i == addr_tmp {
            let mut tmp = 0x08u8;
            let mut epsize = 0x00u8;
            while tmp < size {
                epsize += 1;
                tmp <<= 1;
            }
            usb.ueconx.modify(|_, w| w.epen().clear_bit());
            usb.uecfg1x.modify(|_, w| w.alloc().clear_bit());
            usb.ueconx.modify(|_, w| w.epen().set_bit());
            usb.uecfg0x.modify(|_, w| w.eptype().bits(ep_type));
            if ep & 0x80 != 0 {
                // IN Endpoint
                usb.uecfg0x.modify(|_, w| w.epdir().set_bit());
            }
            usb.uecfg1x
                .write(|w| w.epsize().bits(epsize).alloc().set_bit());
            if double_bank {
                usb.uecfg1x.modify(|_, w| w.epbk().bits(1));
            }
            unsafe {
                usb.ueienx.write(|w| w.bits(0));
            }
        } else {
            if usb.uecfg1x.read().alloc().bit_is_clear() {
                continue;
            }
            let uecfg1x_tmp = usb.uecfg1x.read().bits();
            usb.ueconx.modify(|_, w| w.epen().clear_bit());
            usb.uecfg1x.modify(|_, w| w.alloc().clear_bit());
            usb.ueconx.modify(|_, w| w.epen().set_bit());
            unsafe {
                usb.uecfg1x.write(|w| w.bits(uecfg1x_tmp));
            }
        }

        if usb.uesta0x.read().cfgok().bit_is_clear() {
            // failed
            return false;
        }
    }
    unsafe { usb.uenum.write(|w| w.bits(addr_tmp)) };
    true
}

fn usb_control_request(usb: &USB_DEVICE) {
    let mut buf = [0u8; 64];
    let request = SetupPacket::read(usb);

    match request.bRequest {
        0 => {
            // GET_STATUS
            while usb.ueintx.read().txini().bit_is_clear() {}
            unsafe {
                usb.uedatx.write(|w| w.bits(0));
                usb.uedatx.write(|w| w.bits(0));
            }
            usb.ueintx.modify(|_, w| w.txini().clear_bit());
            return;
        }
        5 => {
            // REQUEST_SET_ADDRESS
            let ty = request.bmRequestType;
            if ty.direction() == Direction::HostToDevice
                && ty.request_type() == Type::Standard
                && ty.recipient() == Recipient::Device
            {
                unsafe { usb.udaddr.write(|w| w.bits((request.wValue & 0x7f) as u8)) };
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
                while usb.ueintx.read().txini().bit_is_clear() {
                    if get_device_status() == DeviceState::Unattached {
                        break;
                    }
                }
                usb.udaddr.modify(|_, w| w.adden().set_bit());
                DEVICE_STATUS.store(DeviceState::Addressed as u8, Ordering::Relaxed);
            }
            return;
        }
        6 => {
            // REQUEST_GET_DESCRIPTOR
            let bytes = match request.wValue >> 8 {
                0x01 => unsafe {
                    let data = DEVICE_DESCR.load();
                    copy_nonoverlapping(
                        &data as *const _ as *const u8,
                        &mut buf as *mut _ as *mut u8,
                        core::mem::size_of::<DeviceDescriptor>(),
                    );
                    &buf[..core::mem::size_of::<DeviceDescriptor>()]
                },
                0x02 => unsafe {
                    let data = CONFIG_DESCR.load();
                    copy_nonoverlapping(
                        &data as *const _ as *const u8,
                        &mut buf as *mut _ as *mut u8,
                        core::mem::size_of::<USBConfiguration>(),
                    );
                    &buf[..core::mem::size_of::<USBConfiguration>()]
                },
                0x03 => {
                    let descr_index = request.wValue & 0xff;
                    if descr_index == 0 {
                        let data = STRING_DESCR0.load();
                        unsafe {
                            copy_nonoverlapping(
                                &data as *const _ as *const u8,
                                &mut buf as *mut _ as *mut u8,
                                data.len(),
                            );
                        }
                        &buf[0..data.len()]
                    } else {
                        let data = STRINGS.load_at(descr_index as usize - 1);
                        let len = build_string_descr(&mut buf, data).unwrap();
                        &buf[0..len]
                    }
                }
                0x21 => unsafe {
                    let data = CONFIG_DESCR.load().hid_func.hid_descriptor;
                    copy_nonoverlapping(
                        &data as *const _ as *const u8,
                        &mut buf as *mut _ as *mut u8,
                        core::mem::size_of::<HidDescriptor>(),
                    );
                    &buf[..core::mem::size_of::<HidDescriptor>()]
                },
                0x22 => unsafe {
                    let data = HID_REPORT_DESCR.load();
                    copy_nonoverlapping(
                        &data as *const u8,
                        &mut buf as *mut _ as *mut u8,
                        data.len(),
                    );
                    &buf[0..data.len()]
                },
                _ => {
                    usb.ueconx
                        .modify(|_, w| w.stallrq().set_bit().epen().set_bit());
                    return;
                    // TODO: なんかステータス調整しなくていい？
                }
            };
            let len = core::cmp::min(core::cmp::min(request.wLength as usize, 255), bytes.len());
            let bytes = &bytes[..len];
            let mut iter = bytes.iter().peekable();
            while iter.peek().is_some() {
                if usb.ueintx.read().rxouti().bit_is_set() {
                    break;
                }
                if usb.ueintx.read().txini().bit_is_set() {
                    for _ in 0..8 {
                        let uebcx = usb.uebclx.read().bits() as u16
                            + ((usb.uebchx.read().bits() as u16) << 8);
                        if let Some(byte) = iter.next() {
                            if uebcx >= 8 {
                                break;
                            }
                            unsafe {
                                usb.uedatx.write(|w| w.bits(*byte));
                            }
                        } else {
                            break;
                        }
                    }
                    usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    while usb.ueintx.read().txini().bit_is_clear() {}
                }
            }
            usb.ueintx.modify(|_, w| w.txini().clear_bit());
            while usb.ueintx.read().rxouti().bit_is_clear() {}
            usb.ueintx
                .modify(|_, w| w.rxouti().clear_bit().fifocon().clear_bit());
            return;
        }
        9 => {
            // REQUEST_SET_CONFIGURATION
            let ty = request.bmRequestType;
            if ty.direction() == Direction::HostToDevice
                && ty.request_type() == Type::Standard
                && ty.recipient() == Recipient::Device
            {
                let cfg = (request.wValue & 0xFF) as u8;
                if cfg > 0 {
                    usb.ueintx
                        .modify(|_, w| w.txini().clear_bit().fifocon().clear_bit());
                    while usb.ueintx.read().txini().bit_is_clear() {
                        if get_device_status() == DeviceState::Unattached {
                            break;
                        }
                    }
                    usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    if configure_endpoint(usb, 0x83, 8, 3, true) {
                        unsafe { usb.uerst.write(|w| w.bits(0x08)) }; // EP3リセット
                        unsafe { usb.uerst.write(|w| w.bits(0)) };
                        DEVICE_STATUS.store(DeviceState::Configured as u8, Ordering::Relaxed);
                    }
                } else {
                    DEVICE_STATUS.store(DeviceState::Addressed as u8, Ordering::Relaxed);
                }
            }
            return;
        }
        _ => {}
    }
    if request.wIndex == 0 {
        // HID request
        match request.bmRequestType.bits() {
            0xa1 => {
                // get requests
                match request.bRequest {
                    0x01 => {
                        // GET_REPORT
                        while usb.ueintx.read().txini().bit_is_clear() {}
                        // TODO: keyboard_modifier
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    }
                    0x02 => {
                        // GET_IDLE
                        while usb.ueintx.read().txini().bit_is_clear() {}
                        let idle_value = KEYBOARD_IDLE_VALUE.load(Ordering::Relaxed);
                        unsafe { usb.uedatx.write(|w| w.bits(idle_value)) };
                        usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    }
                    0x03 => {
                        // GET_PROTOCOL
                        while usb.ueintx.read().txini().bit_is_clear() {}
                        usb.ueintx.modify(|_, w| w.txini().clear_bit());
                        let protocol = KEYBOARD_PROTOCOL.load(Ordering::Relaxed);
                        unsafe { usb.uedatx.write(|w| w.bits(protocol)) };
                        usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    }
                    _ => {}
                }
            }
            0x21 => {
                // set requests
                match request.bRequest {
                    0x09 => {
                        // SET_REPORT
                        while usb.ueintx.read().rxouti().bit_is_clear() {}
                        // TODO: num_lockなど
                        usb.ueintx
                            .modify(|_, w| w.txini().clear_bit().rxouti().clear_bit());
                    }
                    0x0a => {
                        // SET_IDLE
                        KEYBOARD_IDLE_VALUE.store((request.wValue >> 8) as u8, Ordering::Relaxed);
                        usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    }
                    0x0b => {
                        // SET_PROTOCOL
                        KEYBOARD_PROTOCOL.store((request.wValue >> 8) as u8, Ordering::Relaxed);
                        usb.ueintx.modify(|_, w| w.txini().clear_bit());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

progmem! {
    static progmem DEVICE_DESCR: DeviceDescriptor = DeviceDescriptor {
        bLength: core::mem::size_of::<DeviceDescriptor>() as u8,
        bDescriptorType: 1,
        bcdUSB: 0x0200, // USB 2.0
        bDeviceClass: 0,
        bDeviceSubClass: 0,
        bDeviceProtocol: 0,
        bMaxPacketSize0: 8,
        idVendor: 0xfeed,
        idProduct: 0x802f, // ?
        bcdDevice: 0x0100, // v1.00
        iManufacturer: 1,
        iProduct: 2,
        iSerialNumber: 3,
        bNumConfigurations: 1,
    };
    static progmem CONFIG_DESCR: USBConfiguration = USBConfiguration {
        config: ConfigDescriptor {
            bLength: core::mem::size_of::<ConfigDescriptor>() as u8,
            bDescriptorType: 2,
            wTotalLength: core::mem::size_of::<USBConfiguration>() as u16,
            bNumInterfaces: 1,
            bConfigurationValue: 1,
            iConfiguration: 0,
            bmAttributes: 0xC0,
            bMaxPower: 0x32,
        },
        kbd_interf: InterfaceDescriptor {
            bLength: core::mem::size_of::<InterfaceDescriptor>() as u8,
            bDescriptorType: 4,
            bInterfaceNumber: 0,
            bAlternateSetting: 0,
            bNumEndpoints: 0x01,
            bInterfaceClass: 0x03, // Vendor specific
            bInterfaceSubClass: 1,
            bInterfaceProtocol: 1,
            iInterface: 0,
        },
        hid_func: HidFunction {
            hid_descriptor: HidDescriptor {
                bLength: core::mem::size_of::<HidFunction>() as u8,
                bDescriptorType: 0x21,
                bcdHID: 0x0111,
                bCountryCode: 0,
                bNumDescriptors: 1,
            },
            hid_report: HidReport {
                bReportDescriptorType: 0x22,
                wDescriptorLength: 63, // sizeof?
            },
        },
        hid_endpoint: EndpointDescriptor {
            bLength: core::mem::size_of::<EndpointDescriptor>() as u8,
            bDescriptorType: 5,
            bEndpointAddress: 0x83,
            bmAttributes: 0x03,
            wMaxPacketSize: 8,
            bInterval: 0x0a,
        },
    };

    static progmem HID_REPORT_DESCR: [u8; 63] = [
        0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
        0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05, 0x75, 0x01,
        0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x01, 0x95, 0x06,
        0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
    ];

    static progmem STRINGS: [&str; 3] = ["necocen", "necoboard", "17"];
    static progmem STRING_DESCR0: [u8; 4] = [0x04, 0x03, 0x09, 0x04]; // lang id: US English
}

fn build_string_descr(buf: &mut [u8], data: &str) -> Option<usize> {
    let utf16 = data.encode_utf16();

    let iter = buf[2..]
        .chunks_exact_mut(2)
        .zip(utf16)
        .enumerate()
        .map(|(idx, (dst, chr))| {
            dst.copy_from_slice(&chr.to_le_bytes());
            idx
        });
    iter.last().map(|idx| {
        let len = (idx + 1) * 2 + 2;
        buf[0..2].copy_from_slice(&[len as u8, 0x03]);
        len
    })
}

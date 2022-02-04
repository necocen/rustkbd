#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
extern crate panic_halt;

use core::{
    borrow::BorrowMut,
    cell::RefCell,
    cmp::{max, min},
    sync::atomic::{AtomicU8, Ordering},
};

use atmega_hal::{
    clock::MHz16,
    delay::Delay,
    pac, pins,
    port::{mode::Output, Pin, PB5, PB6},
    Peripherals, Pins,
};
use avr_device::interrupt::{self, Mutex};
use avr_progmem::progmem;
use embedded_hal::blocking::delay::DelayMs;

static MY_USB: Mutex<RefCell<Option<pac::USB_DEVICE>>> = Mutex::new(RefCell::new(None));
static MY_PLL: Mutex<RefCell<Option<pac::PLL>>> = Mutex::new(RefCell::new(None));
static MY_B5: Mutex<RefCell<Option<Pin<Output, PB5>>>> = Mutex::new(RefCell::new(None));
static MY_B6: Mutex<RefCell<Option<Pin<Output, PB6>>>> = Mutex::new(RefCell::new(None));
static DEVICE_STATUS: AtomicU8 = AtomicU8::new(DeviceState::Unattached as u8);
static KEYBOARD_PROTOCOL: AtomicU8 = AtomicU8::new(0);
static KEYBOARD_IDLE_VALUE: AtomicU8 = AtomicU8::new(125);

fn get_device_status() -> DeviceState {
    unsafe { core::mem::transmute(DEVICE_STATUS.load(Ordering::Relaxed)) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
#[repr(u8)]
enum DeviceState {
    Unattached = 0u8,
    Reset,
    Powered,
    Suspend,
    Addressed,
}

fn configure_endpoint(dev: &pac::USB_DEVICE, addr: u8, size: u8) -> u8 {
    let addr_tmp = addr & 0x0F;
    for i in addr_tmp..0x07 {
        let mut uecfg0x_tmp = 0u8;
        let mut uecfg1x_tmp = 0u8;
        let mut ueienx_tmp = 0u8;
        unsafe { dev.uenum.write(|w| w.bits(i)) }; //?
        if i == addr_tmp {
            uecfg0x_tmp = 0 << 6; // Type << EPTYPE0
            if (addr & 0x80) != 0 {
                // ENDPOINT_DIR_MASK_IN
                uecfg0x_tmp |= 0x01 << 0 // EPDIR
            }
            // if doubleBank {
            //      uecfg1x_tmp |= 0x01 << 2 // EPBK0
            // }
            let mut tmp = 0x08u8;
            let mut epsize = 0x00u8;
            while tmp < size {
                // ENDPOINT_CONTROL_SIZE
                epsize += 1;
                tmp <<= 1;
            }
            uecfg1x_tmp |= epsize << 4; // EPSIZE0
            uecfg1x_tmp |= 0x01 << 1; // ALLOC
            ueienx_tmp = 0x01 << 3; // RXSTPE
        } else {
            uecfg0x_tmp = dev.uecfg0x.read().bits();
            uecfg1x_tmp = dev.uecfg1x.read().bits();
            ueienx_tmp = dev.ueienx.read().bits();
        }

        if (uecfg1x_tmp & (0x01 << 1)) == 0 {
            // ALLOC
            continue;
        }

        dev.ueconx.modify(|_, w| w.epen().clear_bit());
        dev.uecfg1x.modify(|_, w| w.alloc().clear_bit());
        dev.ueconx.modify(|_, w| w.epen().set_bit());
        unsafe { dev.uecfg0x.write(|w| w.bits(uecfg0x_tmp)) };
        unsafe { dev.uecfg1x.write(|w| w.bits(uecfg1x_tmp)) };
        unsafe { dev.ueienx.write(|w| w.bits(ueienx_tmp)) };

        if dev.uesta0x.read().cfgok().bit_is_clear() {
            // failed
            return 0;
        }
    }
    unsafe { dev.uenum.write(|w| w.bits(addr_tmp)) };
    return 1;
}

#[atmega_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    let dev = dp.USB_DEVICE;
    let pll = dp.PLL;
    let pins = pins!(dp);
    let mut delay = Delay::<MHz16>::new();

    interrupt::free(|cs| {
        MY_USB.borrow(cs).replace(Some(dev));
        MY_PLL.borrow(cs).replace(Some(pll));
        MY_B5.borrow(cs).replace(Some(pins.pb5.into_output()));
        MY_B6.borrow(cs).replace(Some(pins.pb6.into_output()));
    });

    usb_init();

    unsafe {
        interrupt::enable();
    }
    loop {
        //interrupt::free(|cs| MY_B6.borrow(cs).borrow_mut().as_mut().unwrap().toggle());
        // 0.1秒待機
        //delay.delay_ms(100u16);
    }
}

fn usb_init() {
    interrupt::free(|cs| {
        let usb = MY_USB.borrow(cs).borrow();
        let usb = usb.as_ref().unwrap();
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
        // low speed mode (full speed modeのときは外部発振子が必要らしい？)
        usb.udcon.modify(|_, w| w.lsm().set_bit());
        // Only the VBUS interrupt is currently required, as this is used for plug-in detection. This interrupt is activated via the VBUSTE bit in the USBCON register.
        usb.usbcon.modify(|_, w| w.vbuste().set_bit());
        // The DETACH bit in the UDCON register must be cleared so  the selected pull-up resistor is connected to the corresponding data line and the device is detected by the host.
        usb.udcon.modify(|_, w| w.detach().clear_bit());
        // The end of the USB reset can be detected using the EORSTI bit in the UDINT register.
        usb.udien.modify(|_, w| w.eorste().set_bit());
    });
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
            let r = configure_endpoint(usb, 0, 8);
            if r != 0 {
                unsafe { usb.uerst.write(|w| w.bits(1)) };
                unsafe { usb.uerst.write(|w| w.bits(0)) };

                // re-enable receive setup packet interrupt
                usb.ueienx.write(|w| w.rxstpe().set_bit());
                MY_B5.borrow(cs).borrow_mut().as_mut().unwrap().set_high();
                DEVICE_STATUS.store(DeviceState::Reset as u8, Ordering::Relaxed);
            } else {
                MY_B6.borrow(cs).borrow_mut().as_mut().unwrap().set_high();
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
            usb_control_request(
                usb,
                MY_B5.borrow(cs).borrow_mut().as_mut().unwrap(),
                MY_B6.borrow(cs).borrow_mut().as_mut().unwrap(),
            );
        }
        unsafe { usb.uenum.write(|w| w.bits(current_endpoint)) }; // 通らない？
    });
}

#[inline]
fn usb_recv_request(usb: &pac::USB_DEVICE, b5: &mut Pin<Output, PB5>) -> SetupPacket {
    //let packet: SetupPacket = unsafe { core::mem::zeroed() };
    // packet.bmRequestType = BmRequestType(usb.uedatx.read().bits());
    // packet.bRequest = usb.uedatx.read().bits();
    // packet.wValue = usb.uedatx.read().bits() as u16;
    // packet.wValue |= (usb.uedatx.read().bits() as u16) << 8;
    // packet.wIndex = usb.uedatx.read().bits() as u16;
    // packet.wIndex |= (usb.uedatx.read().bits() as u16) << 8;
    // packet.wLength = usb.uedatx.read().bits() as u16;
    // packet.wLength |= (usb.uedatx.read().bits() as u16) << 8;
    // if packet.wValue == 0xffff {
    //     b5.set_low();
    // }

    let mut buf = [0u8; core::mem::size_of::<SetupPacket>()];
    for b in buf.iter_mut() {
        let k = usb.uedatx.read().bits();
        *b = k;
        if ![0x80u8, 0x06u8, 0x01u8, 0x00, 0x12u8, 0x05u8, 0x0eu8, 0x0d].contains(&k) {
            //b5.set_low();
        }
    }
    usb.ueintx.modify(|_, w| w.rxstpi().clear_bit());
    let packet: SetupPacket = unsafe { core::mem::transmute(buf) };
    if ![0x00u8, 0x80].contains(&packet.bmRequestType.bits()) {
        //    b5.set_low();
    }
    if ![0x06u8, 0x05].contains(&packet.bRequest) {
        //    b5.set_low();
    }
    if packet.wValue != 0x000d {
        if packet.wValue == 0x0120 {
            // b5.set_low();
        }
    }

    // buf[0] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[0]) {
    //     b5.set_low();
    // }
    // buf[1] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[1]) {
    //     b5.set_low();
    // }
    // buf[2] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[2]) {
    //     b5.set_low();
    // }
    // buf[3] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[3]) {
    //     b5.set_low();
    // }
    // buf[4] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[4]) {
    //     b5.set_low();
    // }
    // buf[5] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[5]) {
    //     b5.set_low();
    // }
    // buf[6] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[6]) {
    //     b5.set_low();
    // }
    // buf[7] = usb.uedatx.read().bits();
    // if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(&buf[7]) {
    //     b5.set_low();
    // }
    // for b in buf.iter() {
    //     if ![0x80u8, 0x06u8, 0x01u8, 0x00u8, 0x12u8, 0x05u8, 0x0eu8].contains(b) {
    //         //            b5.set_low();
    //     }
    //     if [0x0au8, 0xd1u8].contains(b) {
    //         //b5.set_low();
    //     }
    // }
    //usb.ueintx.modify(|_, w| w.rxstpi().clear_bit());
    //unsafe { core::mem::transmute(buf) }
    packet
}
#[inline(always)]
fn usb_control_request(
    usb: &pac::USB_DEVICE,
    b5: &mut Pin<Output, PB5>,
    b6: &mut Pin<Output, PB6>,
) {
    //let request = usb_recv_request(usb, b5);
    let mut buf = [0u8; 8];
    //let mut delay = Delay::<MHz16>::new();
    for b in buf.iter_mut() {
        *b = usb.uedatx.read().bits();
        //delay.delay_ms(1u16);
    }
    usb.ueintx.modify(|_, w| w.rxstpi().clear_bit());
    //delay.delay_ms(1u16);
    let request: SetupPacket = unsafe { core::mem::transmute(buf) };
    if request.wValue >> 8 == 0x0a {
        b5.set_low();
    } else {
        //b5.set_high();
    }
    //delay.delay_ms(1u16);
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
                //while usb.ueintx.read().txini().bit_is_clear() {}
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
            if request.wValue >> 8 == 0x0a {
                b5.set_low();
            }
            let mut buf = [0u8; 64];
            let bytes = match request.wValue >> 8 {
                0x01 | 0x0a => {
                    //0x01 => {
                    if request.wValue >> 8 == 0x0a {
                        b5.set_low();
                    } else {
                        //b5.set_high();
                    }

                    unsafe {
                        core::slice::from_raw_parts(
                            //(&DEVICE_DESCR.load() as *const DeviceDescriptor) as *const u8,
                            (&DEVICE_DESCR as *const DeviceDescriptor) as *const u8,
                            core::mem::size_of::<DeviceDescriptor>(),
                        )
                    }
                }
                0x02 => {
                    unsafe {
                        core::slice::from_raw_parts(
                            //(&CONFIG_DESCR.load() as *const USBConfiguration) as *const u8,
                            (&CONFIG_DESCR as *const USBConfiguration) as *const u8,
                            core::mem::size_of::<USBConfiguration>(),
                        )
                    }
                }
                0x03 => {
                    let descr_index = request.wValue & 0xff;
                    if descr_index == 0 {
                        //&STRING_DESCR0.load()
                        &STRING_DESCR0
                    } else {
                        //let data = STRINGS.load()[descr_index as usize - 1];
                        let data = STRINGS[descr_index as usize - 1];
                        let len = build_string_descr(&mut buf, data).unwrap();
                        &buf[0..len]
                    }
                }
                0x21 => {
                    unsafe {
                        core::slice::from_raw_parts(
                            //(&CONFIG_DESCR.load().hid_func.hid_descriptor as *const HidDescriptor)
                            (&CONFIG_DESCR.hid_func.hid_descriptor as *const HidDescriptor)
                                as *const u8,
                            core::mem::size_of::<HidDescriptor>(),
                        )
                    }
                }
                //0x22 => &HID_REPORT_DESCR.load(),
                0x22 => &HID_REPORT_DESCR,
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
            //usb.ueintx.modify(|_, w| w.txini().clear_bit());
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
            //MY_B6.borrow(cs).borrow_mut().as_mut().unwrap().set_high();
            // REQUEST_SET_CONFIGURATION
            let ty = request.bmRequestType;
            if ty.direction() == Direction::HostToDevice
                && ty.request_type() == Type::Standard
                && ty.recipient() == Recipient::Device
            {
                let cfg = (request.wValue & 0xFF) as u8;
                usb.ueintx.modify(|_, w| w.txini().clear_bit());
                unsafe { usb.uenum.write(|w| w.bits(3)) }; // KEYBOARD_ENDPOINT_NUM
                usb.ueconx.write(|w| w.epen().set_bit());
                usb.uecfg0x.write(|w| w.eptype().bits(1).epdir().set_bit());
                usb.uecfg1x.write(|w| w.alloc().set_bit().epbk().bits(1));
                unsafe { usb.uerst.write(|w| w.bits(0x1e)) };
                unsafe { usb.uerst.write(|w| w.bits(0)) };
            }
            return;
        }
        _ => {
            //b5.set_low();
            return;
        }
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
                        for _i in 0..6 {
                            // TODO: keyboard_pressed_keys[i]
                            unsafe { usb.uedatx.write(|w| w.bits(0)) };
                        }
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
                        while usb.ueintx.read().txini().bit_is_clear() {}
                        // TODO: keyboard_leds ?
                        usb.ueintx
                            .modify(|_, w| w.txini().clear_bit().rxouti().clear_bit());
                    }
                    0x0a => {
                        // SET_IDLE
                        KEYBOARD_IDLE_VALUE.store(request.wValue as u8, Ordering::Relaxed);
                        // TODO: current_idle = 0;
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

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct DeviceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,
    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
}
#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct ConfigDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub wTotalLength: u16,
    pub bNumInterfaces: u8,
    pub bConfigurationValue: u8,
    pub iConfiguration: u8,
    pub bmAttributes: u8,
    pub bMaxPower: u8,
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct InterfaceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bInterfaceNumber: u8,
    pub bAlternateSetting: u8,
    pub bNumEndpoints: u8,
    pub bInterfaceClass: u8,
    pub bInterfaceSubClass: u8,
    pub bInterfaceProtocol: u8,
    pub iInterface: u8,
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct EndpointDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bEndpointAddress: u8,
    pub bmAttributes: u8,
    pub wMaxPacketSize: u16,
    pub bInterval: u8,
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct HidDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdHID: u16,
    pub bCountryCode: u8,
    pub bNumDescriptors: u8,
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct DebugDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bDebugInEndpoint: u8,
    pub bDebugOutEndpoint: u8,
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct HidReport {
    pub bReportDescriptorType: u8,
    pub wDescriptorLength: u16,
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct HidFunction {
    pub hid_descriptor: HidDescriptor,
    pub hid_report: HidReport,
}
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct USBConfiguration {
    pub config: ConfigDescriptor,
    pub kbd_interf: InterfaceDescriptor,
    pub hid_func: HidFunction,
    pub hid_endpoint: EndpointDescriptor,
}

#[derive(Debug, PartialEq)]
enum Direction {
    HostToDevice,
    DeviceToHost,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum Type {
    Standard,
    Class,
    Vendor,
    Reserved,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum Recipient {
    Device,
    Interface,
    Endpoint,
    Other,
    Reserved,
}
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct BmRequestType(u8);
impl BmRequestType {
    #[inline]
    fn bits(&self) -> u8 {
        self.0
    }

    #[inline]
    fn direction(&self) -> Direction {
        if self.bits() & 0x80 == 0 {
            Direction::HostToDevice
        } else {
            Direction::DeviceToHost
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn request_type(&self) -> Type {
        match (self.bits() >> 5) & 0b11 {
            0 => Type::Standard,
            1 => Type::Class,
            2 => Type::Vendor,
            3 => Type::Reserved,
            _ => unreachable!(),
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn recipient(&self) -> Recipient {
        match self.bits() & 0b11111 {
            0 => Recipient::Device,
            1 => Recipient::Interface,
            2 => Recipient::Endpoint,
            3 => Recipient::Other,
            _ => Recipient::Reserved,
        }
    }
}

#[repr(packed)]
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
pub struct SetupPacket {
    pub bmRequestType: BmRequestType,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
}

// progmem! {
//     static progmem DEVICE_DESCR: DeviceDescriptor = DeviceDescriptor {
//         bLength: core::mem::size_of::<DeviceDescriptor>() as u8,
//         bDescriptorType: 1,
//         bcdUSB: 0x0200, // USB 2.0
//         bDeviceClass: 0,
//         bDeviceSubClass: 0,
//         bDeviceProtocol: 0,
//         bMaxPacketSize0: 8,
//         idVendor: 0xfeed,
//         idProduct: 0x802f, // ?
//         bcdDevice: 0x0100, // v1.00
//         iManufacturer: 1,
//         iProduct: 2,
//         iSerialNumber: 3,
//         bNumConfigurations: 1,
//     };
//     static progmem CONFIG_DESCR: USBConfiguration = USBConfiguration {
//         config: ConfigDescriptor {
//             bLength: core::mem::size_of::<ConfigDescriptor>() as u8,
//             bDescriptorType: 2,
//             wTotalLength: core::mem::size_of::<USBConfiguration>() as u16,
//             bNumInterfaces: 1,
//             bConfigurationValue: 1,
//             iConfiguration: 0,
//             bmAttributes: 0xC0,
//             bMaxPower: 0x32,
//         },
//         kbd_interf: InterfaceDescriptor {
//             bLength: core::mem::size_of::<InterfaceDescriptor>() as u8,
//             bDescriptorType: 4,
//             bInterfaceNumber: 0,
//             bAlternateSetting: 0,
//             bNumEndpoints: 0x01,
//             bInterfaceClass: 0x03, // Vendor specific
//             bInterfaceSubClass: 1,
//             bInterfaceProtocol: 1,
//             iInterface: 0,
//         },
//         hid_func: HidFunction {
//             hid_descriptor: HidDescriptor {
//                 bLength: core::mem::size_of::<HidFunction>() as u8,
//                 bDescriptorType: 0x21,
//                 bcdHID: 0x0111,
//                 bCountryCode: 0,
//                 bNumDescriptors: 1,
//             },
//             hid_report: HidReport {
//                 bReportDescriptorType: 0x22,
//                 wDescriptorLength: 63, // sizeof?
//             },
//         },
//         hid_endpoint: EndpointDescriptor {
//             bLength: core::mem::size_of::<EndpointDescriptor>() as u8,
//             bDescriptorType: 5,
//             bEndpointAddress: 0x83,
//             bmAttributes: 0x03,
//             wMaxPacketSize: 8,
//             bInterval: 0x0a,
//         },
//     };

//     static progmem HID_REPORT_DESCR: &[u8] = &[
//         0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
//         0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05, 0x75, 0x01,
//         0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x01, 0x95, 0x06,
//         0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
//     ];

//     static progmem DEBUG_DESCR: DebugDescriptor = DebugDescriptor {
//         bLength: core::mem::size_of::<DebugDescriptor>() as u8,
//         bDescriptorType: 10,
//         bDebugInEndpoint: 1,
//         bDebugOutEndpoint: 1,
//     };

//     static progmem STRINGS: &[&str; 3] = &["necocen", "necoboard", "17"];
//     static progmem STRING_DESCR0: &[u8] = &[0x04, 0x03, 0x09, 0x04]; // lang id: US English
// }

static DEVICE_DESCR: DeviceDescriptor = DeviceDescriptor {
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
static CONFIG_DESCR: USBConfiguration = USBConfiguration {
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
            bcdHID: 0x0101,
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
        bInterval: 0x01,
    },
};

static DEBUG_DESCR: DebugDescriptor = DebugDescriptor {
    bLength: core::mem::size_of::<DebugDescriptor>() as u8,
    bDescriptorType: 10,
    bDebugInEndpoint: 1,
    bDebugOutEndpoint: 1,
};

static HID_REPORT_DESCR: &[u8] = &[
    0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x05, 0x75, 0x01,
    0x05, 0x08, 0x19, 0x01, 0x29, 0x05, 0x91, 0x02, 0x95, 0x01, 0x75, 0x03, 0x91, 0x01, 0x95, 0x06,
    0x75, 0x08, 0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
];

static STRINGS: &[&str; 3] = &["necocen", "necoboard", "17"];
static STRING_DESCR0: &[u8] = &[0x04, 0x03, 0x09, 0x04]; // lang id: US English

pub fn build_string_descr(buf: &mut [u8], data: &str) -> Option<usize> {
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

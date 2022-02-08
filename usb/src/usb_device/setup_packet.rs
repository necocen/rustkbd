use atmega_hal::pac::USB_DEVICE;

use super::request_type::BmRequestType;

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

impl SetupPacket {
    pub fn read(usb: &USB_DEVICE) -> Self {
        let mut buf = [0u8; core::mem::size_of::<SetupPacket>()];
        for b in buf.iter_mut() {
            *b = usb.uedatx.read().bits();
        }
        usb.ueintx.modify(|_, w| w.rxstpi().clear_bit());
        unsafe { core::mem::transmute(buf) }
    }
}

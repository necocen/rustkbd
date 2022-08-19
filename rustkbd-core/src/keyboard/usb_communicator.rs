use usb_device::{
    class_prelude::{UsbBus, UsbBusAllocator},
    device::{UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid},
    UsbError,
};
use usbd_hid::{
    descriptor::{MediaKeyboardReport, SerializedDescriptor},
    hid_class::HIDClass,
};

use super::{hid_report::HidKeyboardReport, DeviceInfo, Key, NUM_ROLLOVER};

pub struct UsbCommunicator<'a, B: UsbBus> {
    usb_device: UsbDevice<'a, B>,
    keyboard_usb_hid: HIDClass<'a, B>,
    media_usb_hid: HIDClass<'a, B>,
}

impl<'a, B: UsbBus> UsbCommunicator<'a, B> {
    pub fn new(
        device_info: DeviceInfo,
        usb_bus_alloc: &'a UsbBusAllocator<B>,
    ) -> UsbCommunicator<'a, B> {
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

        UsbCommunicator {
            usb_device,
            keyboard_usb_hid,
            media_usb_hid,
        }
    }

    pub fn send_keys(&self, keys: &[Key]) -> Result<(), UsbError> {
        let keyboard_report = keyboard_report(keys);
        let media_key = keys.iter().find(|key| key.is_media_key());
        let media_keyboard_report = media_report(media_key);

        self.keyboard_usb_hid.push_input(&keyboard_report)?;
        self.media_usb_hid.push_input(&media_keyboard_report)?;
        Ok(())
    }

    pub fn poll(&mut self) {
        self.usb_device
            .poll(&mut [&mut self.keyboard_usb_hid, &mut self.media_usb_hid]);
    }

    pub fn state(&self) -> UsbDeviceState {
        self.usb_device.state()
    }
}

fn keyboard_report(keys: &[Key]) -> HidKeyboardReport {
    let mut report = HidKeyboardReport::empty();
    report.modifier = keys
        .iter()
        .map(|key| key.modifier_key_flag())
        .fold(0x00_u8, |acc, flg| acc | flg);
    keys.iter()
        .filter_map(|key| key.key_code())
        .take(NUM_ROLLOVER)
        .enumerate()
        .for_each(|(i, c)| report.key_codes[i] = c);
    report
}

fn media_report(key: Option<&Key>) -> MediaKeyboardReport {
    MediaKeyboardReport {
        usage_id: key.map(|key| key.media_usage_id()).unwrap_or(0),
    }
}

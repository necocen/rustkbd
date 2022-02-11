#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct USBConfiguration {
    pub config: ConfigDescriptor,
    pub kbd_interf: InterfaceDescriptor,
    pub hid_func: HidFunction,
    pub hid_endpoint: EndpointDescriptor,
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

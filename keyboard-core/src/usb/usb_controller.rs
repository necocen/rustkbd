use super::DeviceState;

pub trait UsbController {
    fn get_status(&self) -> DeviceState;
    fn send(&self, data: [u8; 8]);
}

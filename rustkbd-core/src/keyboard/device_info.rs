#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub manufacturer: &'static str,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product_name: &'static str,
    pub serial_number: &'static str,
}

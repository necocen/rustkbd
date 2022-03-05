#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceState {
    Unattached = 0u8,
    Reset,
    Powered,
    Suspend,
    Addressed,
    Configured,
}
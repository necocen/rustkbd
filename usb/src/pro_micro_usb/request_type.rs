#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct BmRequestType(u8);
impl BmRequestType {
    #[inline]
    pub fn bits(&self) -> u8 {
        self.0
    }

    #[inline]
    pub fn direction(&self) -> Direction {
        if self.bits() & 0x80 == 0 {
            Direction::HostToDevice
        } else {
            Direction::DeviceToHost
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn request_type(&self) -> Type {
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
    pub fn recipient(&self) -> Recipient {
        match self.bits() & 0b11111 {
            0 => Recipient::Device,
            1 => Recipient::Interface,
            2 => Recipient::Endpoint,
            3 => Recipient::Other,
            _ => Recipient::Reserved,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Direction {
    HostToDevice,
    DeviceToHost,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum Type {
    Standard,
    Class,
    Vendor,
    Reserved,
}

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum Recipient {
    Device,
    Interface,
    Endpoint,
    Other,
    Reserved,
}

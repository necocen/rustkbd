use core::fmt::Display;

use rp_pico::hal::uart::{Enabled, ReadErrorType, UartDevice, UartPeripheral, ValidUartPinout};
use rustkbd::split::Connection;

pub struct UartConnection<D: UartDevice, P: ValidUartPinout<D>>(pub UartPeripheral<Enabled, D, P>);

impl<D: UartDevice, P: ValidUartPinout<D>> Connection for UartConnection<D, P> {
    fn read_raw(&self, buffer: &mut [u8]) -> nb::Result<usize, ReadError> {
        self.0
            .read_raw(buffer)
            .map_err(|e| e.map(|e| ReadError(e.err_type)))
    }

    fn write(&self, data: &[u8]) {
        self.0.write_full_blocking(data);
    }

    fn read(&self, buffer: &mut [u8]) -> Result<(), ReadError> {
        self.0.read_full_blocking(buffer).map_err(ReadError)
    }

    type Error = ReadError;
}

#[derive(Debug)]
pub struct ReadError(pub ReadErrorType);

impl Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            ReadErrorType::Overrun => f.write_str("ReadError: Overrun"),
            ReadErrorType::Break => f.write_str("ReadError: Break"),
            ReadErrorType::Parity => f.write_str("ReadError: Parity"),
            ReadErrorType::Framing => f.write_str("ReadError: Framing"),
        }
    }
}

impl defmt::Format for ReadError {
    fn format(&self, fmt: defmt::Formatter) {
        match self.0 {
            ReadErrorType::Overrun => defmt::write!(fmt, "ReadError: Overrun"),
            ReadErrorType::Break => defmt::write!(fmt, "ReadError: Break"),
            ReadErrorType::Parity => defmt::write!(fmt, "ReadError: Parity"),
            ReadErrorType::Framing => defmt::write!(fmt, "ReadError: Framing"),
        }
    }
}

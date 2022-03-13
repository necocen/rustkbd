use keyboard_core::split::SplitConnection;
use nb::{Error, Result};
use rp_pico::hal::uart::{Enabled, UartDevice, UartPeripheral, ValidUartPinout};

pub struct UartConnection<D: UartDevice, P: ValidUartPinout<D>>(pub UartPeripheral<Enabled, D, P>);

impl<D: UartDevice, P: ValidUartPinout<D>> SplitConnection for UartConnection<D, P> {
    fn read_raw(&self, buffer: &mut [u8]) -> Result<usize, ()> {
        self.0.read_raw(buffer).map_err(|_| Error::Other(()))
    }

    fn write(&self, data: &[u8]) {
        self.0.write_full_blocking(data);
    }

    fn read(&self, buffer: &mut [u8]) {
        self.0.read_full_blocking(buffer).ok();
    }
}

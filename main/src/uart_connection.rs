use keyboard_core::split_connection::SplitConnection;
use rp_pico::hal::uart::{Enabled, UartDevice, UartPeripheral, ValidUartPinout};

pub struct UartConnection<D: UartDevice, P: ValidUartPinout<D>>(pub UartPeripheral<Enabled, D, P>);

impl<D: UartDevice, P: ValidUartPinout<D>> SplitConnection for UartConnection<D, P> {
    fn read(&self, buffer: &mut [u8]) {
        self.0.read_full_blocking(buffer).ok();
    }

    fn write(&self, data: &[u8]) {
        self.0.write_full_blocking(data);
    }
}

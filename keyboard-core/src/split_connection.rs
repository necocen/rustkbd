use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
use nb;

pub trait SplitConnection {
    fn read_raw(&self, buffer: &mut [u8]) -> nb::Result<usize, ()>;

    fn write(&self, data: &[u8]);

    fn read(&self, buffer: &mut [u8]) {
        let mut offset = 0;
        while offset != buffer.len() {
            offset += match self.read_raw(&mut buffer[offset..]) {
                Ok(bytes_read) => bytes_read,
                Err(e) => match e {
                    nb::Error::Other(_) => return, // TODO: return Err
                    nb::Error::WouldBlock => continue,
                },
            }
        }
    }

    fn read_with_timeout<C: CountDown<Time = Microseconds<u64>>>(
        &self,
        buffer: &mut [u8],
        timer: &mut C,
        timeout: impl Into<Microseconds<u64>>,
    ) -> bool {
        timer.start(timeout);
        let mut offset = 0;
        while offset != buffer.len() {
            if timer.wait().is_ok() {
                return false;
            }
            offset += match self.read_raw(&mut buffer[offset..]) {
                Ok(bytes_read) => bytes_read,
                Err(e) => match e {
                    nb::Error::Other(_) => return false, // TODO: return Err
                    nb::Error::WouldBlock => continue,
                },
            }
        }
        true
    }
}

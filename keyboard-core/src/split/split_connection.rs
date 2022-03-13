use embedded_hal::timer::CountDown;
use heapless::Vec;
use nb;

use super::split_message::SplitMessage;

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

    fn read_with_timeout<C: CountDown>(
        &self,
        buffer: &mut [u8],
        timer: &mut C,
        timeout: impl Into<C::Time>,
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

pub trait SplitConnectionExt: SplitConnection {
    fn read_message<C: CountDown>(
        &self,
        timer: &mut C,
        timeout: impl Into<C::Time>,
    ) -> Option<SplitMessage> {
        let mut buf = [0u8; 16];
        let result = self.read_with_timeout(&mut buf[..1], timer, timeout);
        if !result {
            return None;
        }
        let head = buf[0];
        match head {
            0x00 | 0x01 => {
                let ctor = if head == 0x00 {
                    SplitMessage::KeyInput
                } else {
                    SplitMessage::KeyInputReply
                };
                self.read(&mut buf[..1]);
                let len = buf[0] as usize;
                if len == 0 {
                    Some(ctor(Vec::new()))
                } else if len > 8 {
                    None // TODO: エラー
                } else {
                    self.read(&mut buf[..(len * 2)]);
                    let keys: Vec<(u8, u8), 6> = (0..len)
                        .map(|x| x * 2)
                        .map(|x| (buf[x], buf[x + 1]))
                        .collect();
                    Some(ctor(keys))
                }
            }
            0xff => Some(SplitMessage::FindReceiver),
            0xfe => Some(SplitMessage::Acknowledge),
            _ => None, // TODO: エラーにすべき？
        }
    }

    fn send_message(&self, message: SplitMessage) {
        match message {
            SplitMessage::KeyInput(ref keys) | SplitMessage::KeyInputReply(ref keys) => {
                let head = if let SplitMessage::KeyInput(_) = message {
                    0x00
                } else {
                    0x01
                };
                let len = keys.len() as u8;
                let data = core::iter::once(head)
                    .chain(core::iter::once(len))
                    .chain(keys.iter().flat_map(|(col, row)| [*col, *row]))
                    .collect::<Vec<u8, 18>>();
                self.write(&data);
            }
            SplitMessage::Acknowledge => {
                self.write(&[0xfe]);
            }
            SplitMessage::FindReceiver => {
                self.write(&[0xff]);
            }
        }
    }
}

impl<T: SplitConnection> SplitConnectionExt for T {}

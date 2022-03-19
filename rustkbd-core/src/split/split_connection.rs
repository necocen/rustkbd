use embedded_hal::timer::CountDown;
use heapless::Vec;
use nb;

use crate::keyboard::KeySwitchIdentifier;

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

/// 一度に書き込む最大バイト数
const MAX_BUF_LEN: usize = 40;

pub trait SplitConnectionExt: SplitConnection {
    fn read_message<C: CountDown, const SZ: usize, const RO: usize, SI: KeySwitchIdentifier<SZ>>(
        &self,
        timer: &mut C,
        timeout: impl Into<C::Time>,
    ) -> Option<SplitMessage<SZ, RO, SI>> {
        assert!(
            MAX_BUF_LEN > SZ * RO,
            "MAX_BUF_LEN must be large enough to read SI bytes x RO keys"
        );
        let mut buf = [0u8; MAX_BUF_LEN];
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
                } else if len > RO {
                    None // TODO: エラー
                } else {
                    self.read(&mut buf[..(len * SZ)]);
                    let keys = (0..len)
                        .map(|x| x * SZ)
                        .map(|x| {
                            let mut b: [u8; SZ] = [0; SZ];
                            b.copy_from_slice(&buf[x..(x + SZ)]);
                            b.into()
                        })
                        .collect();
                    Some(ctor(keys))
                }
            }
            0xff => Some(SplitMessage::FindReceiver),
            0xfe => Some(SplitMessage::Acknowledge),
            _ => None, // TODO: エラーにすべき？
        }
    }

    fn send_message<const SZ: usize, const RO: usize, SI: KeySwitchIdentifier<SZ>>(
        &self,
        message: SplitMessage<SZ, RO, SI>,
    ) {
        assert!(
            MAX_BUF_LEN > SZ * RO,
            "MAX_BUF_LEN must be large enough to write SI bytes x RO keys"
        );
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
                    .chain(
                        keys.into_iter()
                            .flat_map::<[u8; SZ], _>(|key| (*key).into()),
                    )
                    .collect::<Vec<u8, MAX_BUF_LEN>>();
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

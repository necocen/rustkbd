use core::fmt::Debug;

use embedded_hal::timer::CountDown;
use nb;

use crate::{
    keyboard::KeySwitchIdentifier,
    split::{Error, Message},
    Vec,
};

pub trait Connection {
    type Error: 'static + defmt::Format + Debug;
    fn read_raw(&self, buffer: &mut [u8]) -> nb::Result<usize, Self::Error>;

    fn write(&self, data: &[u8]);

    fn read(&self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let mut offset = 0;
        while offset != buffer.len() {
            offset += match self.read_raw(&mut buffer[offset..]) {
                Ok(bytes_read) => bytes_read,
                Err(e) => match e {
                    nb::Error::Other(source) => return Err(source),
                    nb::Error::WouldBlock => continue,
                },
            }
        }
        Ok(())
    }
}

/// 一度に書き込む最大バイト数
const MAX_BUF_LEN: usize = 40;

pub trait ConnectionExt: Connection {
    fn read_message<C: CountDown, const SZ: usize, const RO: usize, SI: KeySwitchIdentifier<SZ>>(
        &self,
        timer: &mut C,
        timeout: impl Into<C::Time>,
    ) -> Result<Message<SZ, RO, SI>, Error<Self::Error>> {
        assert!(
            MAX_BUF_LEN > SZ * RO,
            "MAX_BUF_LEN must be large enough to read SI bytes x RO keys"
        );
        let mut buf = [0u8; MAX_BUF_LEN];
        self.read_with_timeout(&mut buf[..1], timer, timeout)?;
        let head = buf[0];
        match head {
            0x00 | 0x01 => {
                let ctor = if head == 0x00 {
                    Message::Switches
                } else {
                    Message::SwitchesReply
                };
                self.read(&mut buf[..1])
                    .map_err(|source| Error::ReadError { source })?;
                let len = buf[0] as usize;
                if len == 0 {
                    Ok(ctor(Vec::new()))
                } else if len > RO {
                    Err(Error::ReadBufferOverflow)
                } else {
                    self.read(&mut buf[..(len * SZ)])
                        .map_err(|source| Error::ReadError { source })?;
                    let keys = (0..len)
                        .map(|x| x * SZ)
                        .map(|x| {
                            let mut b: [u8; SZ] = [0; SZ];
                            b.copy_from_slice(&buf[x..(x + SZ)]);
                            b.into()
                        })
                        .collect();
                    Ok(ctor(keys))
                }
            }
            0xff => Ok(Message::FindReceiver),
            0xfe => Ok(Message::Acknowledge),
            _ => Err(Error::UnknownMessage { head }),
        }
    }

    fn send_message<const SZ: usize, const RO: usize, SI: KeySwitchIdentifier<SZ>>(
        &self,
        message: Message<SZ, RO, SI>,
    ) {
        assert!(
            MAX_BUF_LEN > SZ * RO,
            "MAX_BUF_LEN must be large enough to write SI bytes x RO keys"
        );
        match message {
            Message::Switches(ref keys) | Message::SwitchesReply(ref keys) => {
                let head = if let Message::Switches(_) = message {
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
            Message::Acknowledge => {
                self.write(&[0xfe]);
            }
            Message::FindReceiver => {
                self.write(&[0xff]);
            }
        }
    }
    fn read_with_timeout<C: CountDown>(
        &self,
        buffer: &mut [u8],
        timer: &mut C,
        timeout: impl Into<C::Time>,
    ) -> Result<(), Error<Self::Error>> {
        timer.start(timeout);
        let mut offset = 0;
        while offset != buffer.len() {
            if timer.wait().is_ok() {
                return Err(Error::ReadTimedOut);
            }
            offset += match self.read_raw(&mut buffer[offset..]) {
                Ok(bytes_read) => bytes_read,
                Err(e) => match e {
                    nb::Error::Other(source) => return Err(Error::ReadError { source }),
                    nb::Error::WouldBlock => continue,
                },
            }
        }
        Ok(())
    }
}

impl<T: Connection> ConnectionExt for T {}

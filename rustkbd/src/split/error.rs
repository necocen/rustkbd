use core::fmt::Debug;

use defmt::Format;

#[derive(Debug, Format)]
pub enum Error<E: 'static + Debug> {
    ReadTimedOut,
    ReadBufferOverflow,
    ReadError {
        #[defmt(Debug2Format)]
        source: E,
    },
    UnknownMessage {
        head: u8,
    },
}

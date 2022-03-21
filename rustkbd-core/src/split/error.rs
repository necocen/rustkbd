use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error<E: 'static + snafu::Error> {
    #[snafu(display("Read from connection timed out"))]
    ReadTimedOut,
    #[snafu(display("Read buffer overflowed"))]
    ReadBufferOverflow,
    #[snafu(display("Read error: {source}"))]
    ReadError { source: E },
    #[snafu(display("Unknown message with type {head}"))]
    UnknownMessage { head: u8 },
}

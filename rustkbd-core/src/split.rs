mod connection;
mod error;
mod message;
mod split_state;
pub(crate) use connection::ConnectionExt;
pub use connection::{Connection, DummyConnection};
pub use error::Error;
pub(crate) use message::Message;
pub use split_state::SplitState;

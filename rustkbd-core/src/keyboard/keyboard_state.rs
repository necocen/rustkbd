#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardState {
    Undetermined,
    WaitingForReceiver,
    Controller,
    Receiver,
}

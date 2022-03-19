#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitState {
    Undetermined,
    WaitingForReceiver,
    Controller,
    Receiver,
}

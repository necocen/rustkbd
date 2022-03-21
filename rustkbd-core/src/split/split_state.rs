#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitState {
    Undetermined,
    Controller,
    Receiver,
}

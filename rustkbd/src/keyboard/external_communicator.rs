use super::Key;

pub trait ExternalCommunicator {
    type Error;
    fn is_ready(&self) -> bool;
    fn send_keys(&self, keys: &[Key]) -> Result<(), Self::Error>;
}

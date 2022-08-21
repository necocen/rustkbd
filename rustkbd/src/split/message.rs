use crate::{keyboard::KeySwitchIdentifier, Vec};

#[derive(Debug, Clone)]
pub enum Message<const SZ: usize, const RO: usize, SI: KeySwitchIdentifier<SZ>> {
    Switches(Vec<SI, RO>),      // 0x00
    SwitchesReply(Vec<SI, RO>), // 0x01
    Acknowledge,                // 0xfe
    FindReceiver,               // 0xff
}

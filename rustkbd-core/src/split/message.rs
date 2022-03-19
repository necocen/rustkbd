use heapless::Vec;

use crate::keyboard::KeySwitchIdentifier;

#[derive(Debug, Clone)]
pub enum Message<const SZ: usize, const RO: usize, SI: KeySwitchIdentifier<SZ>> {
    KeyInput(Vec<SI, RO>),      // 0x00
    KeyInputReply(Vec<SI, RO>), // 0x01
    Acknowledge,                // 0xfe
    FindReceiver,               // 0xff
}

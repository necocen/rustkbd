use heapless::Vec;

use crate::key_switches::KeySwitchIdentifier;

#[derive(Debug, Clone)]
pub enum SplitMessage<const SZ: usize, SI: KeySwitchIdentifier<SZ>> {
    KeyInput(Vec<SI, 6>),      // 0x00
    KeyInputReply(Vec<SI, 6>), // 0x01
    Acknowledge,               // 0xfe
    FindReceiver,              // 0xff
}

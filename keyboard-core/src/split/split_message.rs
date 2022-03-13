use heapless::Vec;

#[derive(Debug, Clone)]
pub enum SplitMessage {
    KeyInput(Vec<(u8, u8), 6>),      // 0x00
    KeyInputReply(Vec<(u8, u8), 6>), // 0x01
    Acknowledge,                     // 0xfe
    FindReceiver,                    // 0xff
}

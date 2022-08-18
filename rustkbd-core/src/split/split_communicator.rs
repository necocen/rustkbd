use embedded_hal::timer::CountDown;
use embedded_time::duration::Microseconds;
use heapless::Vec;

use crate::keyboard::{KeySwitches, NUM_SWITCH_ROLLOVER};

use super::{Connection, ConnectionExt, Error, Message, SplitState};

pub struct SplitCommunicator<
    const SZ: usize,
    K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
    S: Connection,
    T: CountDown<Time = Microseconds<u64>>,
> {
    connection: S,
    state: SplitState,
    timer: T,
    buffer: Vec<K::Identifier, NUM_SWITCH_ROLLOVER>,
}

impl<
        const SZ: usize,
        K: KeySwitches<SZ, NUM_SWITCH_ROLLOVER>,
        S: Connection,
        T: CountDown<Time = Microseconds<u64>>,
    > SplitCommunicator<SZ, K, S, T>
{
    pub fn new(connection: S, timer: T) -> SplitCommunicator<SZ, K, S, T> {
        SplitCommunicator {
            connection,
            state: SplitState::Undetermined, // TODO: これだとNotAvailableになれない
            timer,
            buffer: Vec::new(),
        }
    }

    pub fn establish(&mut self) -> Result<(), Error<S::Error>> {
        self.state = SplitState::Undetermined;
        self.connection
            .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::FindReceiver);
        self.state = match self.read()? {
            Message::Acknowledge => {
                defmt::info!("Split connection established");
                SplitState::Controller
            }
            _ => {
                defmt::warn!("Unexpected response");
                SplitState::Undetermined
            }
        };
        Ok(())
    }

    pub fn state(&self) -> SplitState {
        self.state
    }

    pub fn respond(&mut self, keys: &Vec<K::Identifier, NUM_SWITCH_ROLLOVER>) {
        match self.read() {
            Ok(Message::Switches(switches)) => {
                // Controllerからrequestが届いたとき：バッファに保存しつつkeysをReplyする
                self.buffer = switches;
                self.connection
                    .send_message(Message::SwitchesReply(keys.clone()));
            }
            Ok(Message::SwitchesReply(switches)) => {
                // 通常ここには来ないがタイミングの問題で来る場合があるので適切にハンドリングする
                self.buffer = switches;
            }
            Ok(Message::FindReceiver) => {
                // Controllerからestablishが届いたとき：Acknowledgeを応答して自分をReceiverにする
                self.connection
                    .send_message(Message::<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>::Acknowledge);
                self.state = SplitState::Receiver;
            }
            Ok(_) => {
                defmt::warn!("Received unexpected message");
            }
            Err(e) => {
                defmt::warn!("Failed to receive request: {}", e);
            }
        }
    }

    pub fn request(
        &mut self,
        keys: &Vec<K::Identifier, NUM_SWITCH_ROLLOVER>,
    ) -> Vec<K::Identifier, NUM_SWITCH_ROLLOVER> {
        // Controllerの場合：自分のキーを送信、Receiverから応答を受信して返す
        // Receiverの場合：respondで受信していたバッファを返す
        // それ以外：空
        match self.state {
            SplitState::Controller => {
                self.connection
                    .send_message(Message::Switches(keys.clone()));
                match self.read() {
                    Ok(Message::SwitchesReply(switches)) => {
                        // replied
                        self.buffer = switches.clone();
                        switches
                    }
                    Ok(_) => {
                        defmt::warn!("Received unexpected reply");
                        self.buffer.clone()
                    }
                    Err(e) => {
                        defmt::warn!("Failed to receive request: {}", e);
                        self.buffer.clone()
                    }
                }
            }
            SplitState::Receiver => self.buffer.clone(),
            _ => Vec::new(),
        }
    }

    fn read(&mut self) -> Result<Message<SZ, NUM_SWITCH_ROLLOVER, K::Identifier>, Error<S::Error>> {
        self.connection
            .read_message(&mut self.timer, Microseconds::<u64>::new(10_000)) // timeout in 10ms
    }
}

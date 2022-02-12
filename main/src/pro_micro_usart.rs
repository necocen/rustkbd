use core::{cell::RefCell, iter::once};

use atmega_hal::pac::USART1;
use avr_device::interrupt::{self, Mutex};
use heapless::{spsc::Queue, Vec};
use keyboard_core::usart::UsartController;

static MY_USART1: Mutex<RefCell<Option<USART1>>> = Mutex::new(RefCell::new(None));
static QUEUE: Mutex<RefCell<Queue<u8, 32>>> = Mutex::new(RefCell::new(Queue::new()));

#[derive(Debug)]
#[non_exhaustive]
pub struct ProMicroUsart {}

impl ProMicroUsart {
    pub fn new(usart1: USART1) -> Self {
        interrupt::free(|cs| {
            // baud rate 19200 bps
            unsafe { usart1.ubrr1.write(|w| w.bits(51)) };
            // 受信許可・受信割り込み許可・送信許可
            usart1
                .ucsr1b
                .write(|w| w.rxen1().set_bit().rxcie1().set_bit().txen1().set_bit());
            // 8bits・非同期・パリティなし
            usart1
                .ucsr1c
                .write(|w| w.ucsz1().chr8().umsel1().usart_async().upm1().disabled());
            MY_USART1.borrow(cs).replace(Some(usart1));
        });
        ProMicroUsart {}
    }
}

impl UsartController for ProMicroUsart {
    type KeySwitchId = (u8, u8);

    fn get(&mut self) -> Option<Vec<Self::KeySwitchId, 6>> {
        interrupt::free(|cs| {
            let mut queue = QUEUE.borrow(cs).borrow_mut();
            let mut vec = Vec::<Self::KeySwitchId, 6>::new();
            if let Some(len) = queue.peek() {
                let len = (len & 0x7f) as usize;
                if queue.len() >= len * 2 + 1 {
                    queue.dequeue(); // len
                    for _ in 0..len {
                        let col = queue.dequeue().unwrap();
                        let row = queue.dequeue().unwrap();
                        vec.push((col, row)).ok();
                    }
                    Some(vec)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn put(&mut self, keys: &[Self::KeySwitchId]) {
        interrupt::free(|cs| {
            let usart1 = MY_USART1.borrow(cs).borrow();
            let usart1 = usart1.as_ref().unwrap();
            while usart1.ucsr1a.read().udre1().bit_is_clear() {}
            let len = (keys.len() as u8) | 0x80;
            unsafe {
                usart1.udr1.write(|w| w.bits(len));
            }
            let iter = keys
                .into_iter()
                .flat_map(|(col, row)| once(col).chain(once(row)));
            for byte in iter {
                while usart1.ucsr1a.read().udre1().bit_is_clear() {}
                unsafe {
                    usart1.udr1.write(|w| w.bits(*byte));
                }
            }
        });
    }
}

#[avr_device::interrupt(atmega32u4)]
fn USART1_RX() {
    interrupt::free(|cs| {
        let mut queue = QUEUE.borrow(cs).borrow_mut();
        let usart = MY_USART1.borrow(cs).borrow();
        let usart = usart.as_ref().unwrap();
        let byte = usart.udr1.read().bits();
        queue.enqueue(byte).ok();
    });
}

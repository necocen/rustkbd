#[derive(Debug, Clone)]
pub struct Buffer<const N: usize> {
    buf: [u8; N],
    pos: usize,
}

impl<const N: usize> Buffer<N> {
    pub fn new() -> Self {
        Buffer {
            buf: [0; N],
            pos: 0,
        }
    }

    pub fn update(&mut self, value: bool) -> bool {
        self.buf[self.pos] = value as u8;
        self.pos = (self.pos + 1) % N;
        self.buf.iter().copied().sum::<u8>() * 2 >= (N as u8)
    }
}

pub trait SplitConnection {
    fn read(&self, buffer: &mut [u8]);

    fn write(&self, data: &[u8]);
}

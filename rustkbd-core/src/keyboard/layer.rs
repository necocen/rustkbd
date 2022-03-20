pub trait Layer: Copy + Eq + Default {
    fn below(&self) -> Option<Self>;
}

pub use rustkbd_derive::KeyboardLayer;

pub trait KeyboardLayer: Copy + Eq + Default {
    fn below(&self) -> Option<Self>;
}

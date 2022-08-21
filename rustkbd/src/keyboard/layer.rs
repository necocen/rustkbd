pub use rustkbd_derive::Layer;

pub trait Layer: Copy + Eq + Default {
    fn below(&self) -> Option<Self>;
}

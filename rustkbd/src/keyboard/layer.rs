pub use rustkbd_macros::Layer;

pub trait Layer: Copy + Eq + Default {
    fn below(&self) -> Option<Self>;
}

use heapless::Vec;

use super::{Key, Layer};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct KeyboardState<L: Layer, const RO: usize> {
    pub layer: L,
    pub keys: Vec<Key, RO>,
}

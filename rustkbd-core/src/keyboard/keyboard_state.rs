use super::{Key, KeyboardLayer, Vec};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct KeyboardState<L: KeyboardLayer, const RO: usize> {
    pub layer: L,
    pub keys: Vec<Key, RO>,
}

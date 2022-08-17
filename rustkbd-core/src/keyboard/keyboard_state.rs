use super::{Key, KeyboardLayer, Vec};
use crate::split::SplitState;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct KeyboardState<L: KeyboardLayer, const RO: usize> {
    pub layer: L,
    pub keys: Vec<Key, RO>,
    pub split: SplitState,
}

impl<L: KeyboardLayer, const RO: usize> KeyboardState<L, RO> {
    pub fn is_controller(&self) -> bool {
        self.split == SplitState::Controller
    }

    pub fn is_split_undetermined(&self) -> bool {
        self.split == SplitState::Undetermined
    }

    pub fn is_not_splitted(&self) -> bool {
        self.split == SplitState::NotAvailable
    }
}

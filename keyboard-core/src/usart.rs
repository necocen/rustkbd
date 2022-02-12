use heapless::Vec;

pub trait UsartController {
    type KeySwitchId: Copy + Sized;
    fn get(&mut self) -> Option<Vec<Self::KeySwitchId, 6>>;

    fn put(&mut self, keys: &[Self::KeySwitchId]);
}

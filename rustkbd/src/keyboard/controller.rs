use heapless::{FnvIndexMap, Vec};

use super::{
    ExternalCommunicator, Key, KeySwitchIdentifier, KeySwitches, KeyboardState, Layer, Layout,
};

pub struct Controller<
    const SZ: usize,
    const RO: usize,
    C: ExternalCommunicator,
    K: KeySwitches<SZ, RO>,
    L: Layout<SZ, Identifier = K::Identifier>,
> {
    pub communicator: C,
    pub key_switches: K,
    layer: L::Layer,
    layout: L,
    keys: Vec<Key, RO>,
    pressed_switches: FnvIndexMap<K::Identifier, L::Layer, 16>,
}

impl<
        const SZ: usize,
        const RO: usize,
        C: ExternalCommunicator,
        K: KeySwitches<SZ, RO>,
        L: Layout<SZ, Identifier = K::Identifier>,
    > Controller<SZ, RO, C, K, L>
{
    pub fn new(communicator: C, key_switches: K, layout: L) -> Self {
        Controller {
            communicator,
            key_switches,
            layer: L::Layer::default(),
            layout,
            keys: Vec::new(),
            pressed_switches: FnvIndexMap::new(),
        }
    }

    pub fn get_state(&self) -> KeyboardState<L::Layer, RO> {
        KeyboardState {
            layer: self.layer,
            keys: self.keys.clone(),
        }
    }

    pub fn main_loop(&mut self) {
        defmt::info!("0");
        let switches = self.key_switches.scan();

        defmt::info!("1");
        // グローバルなレイヤの決定
        let global_layer = self.layout.layer(&switches);

        defmt::info!("2");
        // 個別のスイッチのレイヤの決定
        let switches_and_layers: Vec<_, RO> =
            determine_layers(&self.pressed_switches, &switches, global_layer);

            defmt::info!("3");
        // キーの決定
        let keys = determine_keys(&self.layout, &switches_and_layers);
        defmt::info!("4");
        let keys = filter_keys(keys);
        defmt::info!("5");

        if !keys.is_empty() {
            defmt::debug!("{}", keys.as_slice());
        }

        // スイッチ押下状態の更新
        self.pressed_switches = switches_and_layers
            .into_iter()
            .map(|(s, l)| (*s, l))
            .collect();

            defmt::info!("6");
        self.layer = global_layer;
        self.keys = keys;
    }

    pub fn send_keys(&self) -> Result<(), C::Error> {
        if !self.communicator.is_ready() {
            return Ok(());
        }

        self.communicator.send_keys(&self.keys)
    }
}

fn determine_layers<
    'a,
    Y: Layer,
    SI: KeySwitchIdentifier<SZ>,
    const SZ: usize,
    const RO: usize,
    const N: usize,
>(
    pressed_switches: &FnvIndexMap<SI, Y, N>,
    switches: &'a [SI],
    global_layer: Y,
) -> Vec<(&'a SI, Y), RO> {
    // 個別のスイッチのレイヤの決定
    switches
        .iter()
        .map(|s| {
            let layer = if let Some(layer) = pressed_switches.get(s) {
                *layer
            } else {
                global_layer
            };
            (s, layer)
        })
        .collect()
}

fn determine_keys<L: Layout<SZ>, const SZ: usize, const RO: usize>(
    layout: &L,
    switches_and_layers: &[(&L::Identifier, L::Layer)],
) -> Vec<Key, RO> {
    switches_and_layers
        .iter()
        .map(|(switch, mut layer)| {
            let mut key = layout.key(layer, switch);
            while key == Key::Transparent {
                if let Some(below) = layer.below() {
                    assert!(
                        layer != below,
                        "{}.below() does not change layer",
                        stringify!(L::Layer)
                    );
                    layer = below;
                    key = layout.key(layer, switch);
                } else {
                    break;
                }
            }
            key
        })
        .filter(|key| !key.is_noop())
        .collect::<Vec<Key, RO>>()
}

fn filter_keys<const RO: usize>(mut keys: Vec<Key, RO>) -> Vec<Key, RO> {
    if keys
        .iter()
        .any(|k| !k.is_modified_key() && !k.is_modifier_key())
    {
        // 修飾済みキー以外が押されているときは、修飾済みキーは無効化する
        keys.retain(|k| !k.is_modified_key());
        // FIXME: シフトキーが押下されている状態でシフト修飾済みキーと非修飾キーが押下されたときは、シフト修飾済みキーを生かすべき
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // 修飾キーと関係のない場合
    fn test_filter_keys_no_modified_keys() {
        let mut keys = Vec::<Key, 6>::new();
        keys.push(Key::A).unwrap();
        keys.push(Key::B).unwrap();
        let mut expected = Vec::<Key, 6>::new();
        expected.push(Key::A).unwrap();
        expected.push(Key::B).unwrap();
        assert_eq!(expected, filter_keys(keys));
    }

    #[test]
    // 修飾済みキーと非修飾キーが混在する場合、修飾済みキーは除外される
    fn test_filter_keys_with_modified_key_and_unmodified_keys() {
        let mut keys = Vec::<Key, 6>::new();
        keys.push(Key::Asterisk).unwrap();
        keys.push(Key::A).unwrap();
        keys.push(Key::B).unwrap();
        let mut expected = Vec::<Key, 6>::new();
        expected.push(Key::A).unwrap();
        expected.push(Key::B).unwrap();
        assert_eq!(expected, filter_keys(keys));
    }

    #[test]
    // 修飾済みキーと修飾キーの混在は許容される
    fn test_filter_keys_with_modified_key_and_modifier_keys() {
        let mut keys = Vec::<Key, 6>::new();
        keys.push(Key::Asterisk).unwrap();
        keys.push(Key::LeftGui).unwrap();
        let mut expected = Vec::<Key, 6>::new();
        expected.push(Key::Asterisk).unwrap();
        expected.push(Key::LeftGui).unwrap();
        assert_eq!(expected, filter_keys(keys));
    }

    #[test]
    // 修飾済みキーと修飾キーと非修飾キーが混在する場合は、修飾キーと非修飾キーが残る
    fn test_filter_keys_with_modified_key_and_modifier_keys_and_unmodified_keys() {
        let mut keys = Vec::<Key, 6>::new();
        keys.push(Key::Asterisk).unwrap();
        keys.push(Key::LeftGui).unwrap();
        keys.push(Key::A).unwrap();
        keys.push(Key::B).unwrap();
        let mut expected = Vec::<Key, 6>::new();
        expected.push(Key::LeftGui).unwrap();
        expected.push(Key::A).unwrap();
        expected.push(Key::B).unwrap();
        assert_eq!(expected, filter_keys(keys));
    }
}

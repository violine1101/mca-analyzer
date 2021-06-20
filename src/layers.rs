use std::collections::HashMap;

pub struct Layer {
    composition: HashMap<String, u32>,
    pub y: i32,
}

impl Layer {
    pub fn get_count(&self, blockstate: &str) -> u32 {
        *self.composition.get(blockstate).unwrap_or(&0)
    }

    pub fn increment(&mut self, blockstate: &str) {
        let prev_count = self.get_count(blockstate);
        self.composition
            .insert(blockstate.to_string(), prev_count + 1);
    }
}

pub struct Layers {
    layers: HashMap<i32, Layer>,
}

impl Layers {
    pub fn new() -> Self {
        Layers {
            layers: HashMap::new(),
        }
    }

    pub fn increment(&mut self, blockstate: &str, layer: i32) {
        if let Some(layer) = self.layers.get_mut(&layer) {
            layer.increment(blockstate);
        } else {
            let composition = vec![(blockstate.to_string(), 1)].into_iter().collect();
            self.layers.insert(
                layer,
                Layer {
                    composition,
                    y: layer,
                },
            );
        }
    }
}

impl IntoIterator for Layers {
    type Item = Layer;

    type IntoIter = std::vec::IntoIter<Layer>;

    fn into_iter(self) -> Self::IntoIter {
        let mut list = self.layers.into_iter().collect::<Vec<_>>();
        list.sort_by(|a, b| a.0.cmp(&b.0));

        let list = list.into_iter().map(|(_, layer)| layer).collect::<Vec<_>>();

        list.into_iter()
    }
}

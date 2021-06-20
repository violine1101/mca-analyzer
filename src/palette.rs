use std::{cmp::max, collections::HashMap, convert::TryInto};

use nbt::CompoundTag;

pub struct Palette {
    indices: HashMap<String, usize>,
    elements: Vec<String>,
}

impl Palette {
    pub fn new() -> Self {
        Palette {
            indices: HashMap::new(),
            elements: Vec::new(),
        }
    }

    // We can't reuse `add` from below because chunk palettes allow for duplicates
    // (because they differentiate between block states, which we ignore here)
    pub fn from_nbt(nbt: Vec<&CompoundTag>) -> Self {
        let mut indices = HashMap::<String, usize>::with_capacity(nbt.len());
        let mut elements = Vec::with_capacity(nbt.len());

        nbt.into_iter().enumerate().for_each(|(index, entry)| {
            let blockstate = parse_palette_entry(entry);

            // Vanilla minecraft will implicitly have id 0 = minecraft:air
            // even if it's not specified
            if index == 0 && blockstate != "minecraft:air" {
                indices.insert("minecraft:air".to_string(), 0);
                elements.push(blockstate.to_string());
            }

            indices.insert(blockstate.to_string(), elements.len());
            elements.push(blockstate.to_string());
        });

        Palette { indices, elements }
    }

    // Calculate amount of bits for each palette item in the .mca format
    pub fn get_elem_bit_size(&self) -> u32 {
        let palette_length: i32 = self.elements.len().try_into().unwrap();
        max(4, f64::log2(palette_length.into()).ceil() as u32)
    }

    pub fn add(&mut self, name: &str) -> usize {
        if let Some(&index) = self.indices.get(name) {
            index
        } else {
            let index = self.elements.len();

            self.indices.insert(name.to_string(), index);
            self.elements.push(name.to_string());

            index
        }
    }

    pub fn get_state(&self, id: usize) -> Option<&str> {
        self.elements.get(id).map(|s| s.as_str())
    }
}

fn parse_palette_entry(palette_entry: &CompoundTag) -> &str {
    palette_entry
        .get_str("Name")
        .expect("Couldn't get field Name for palette entry")
}

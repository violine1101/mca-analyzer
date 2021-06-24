use std::collections::HashMap;

use crate::{
    area::Area,
    chunk_loader::ChunkLoader,
    chunk_section::{ChunkSection, ChunkSectionBlock},
    layers::Layers,
};

pub struct CompositionAnalyzer<'a> {
    blockstate_map: HashMap<String, u32>,
    layers: Layers,
    chunk_loader: ChunkLoader<'a>,
}

impl<'a> CompositionAnalyzer<'a> {
    pub fn new(path: &'a str) -> Self {
        CompositionAnalyzer {
            blockstate_map: HashMap::new(),
            layers: Layers::new(),
            chunk_loader: ChunkLoader::new(path),
        }
    }

    pub fn analyze(&mut self, area: Area) {
        for (chunk_x, chunk_z) in area {
            let chunk = self.chunk_loader.get_or_load(chunk_x, chunk_z).clone();

            eprintln!("Analyzing chunk ({},{})", chunk_x, chunk_z);

            for section in chunk {
                self.count_chunk_section(section);
            }
        }
    }

    fn count_blockstate(&mut self, block: ChunkSectionBlock) {
        let blockstate = block.blockstate;

        let prev_blockstate_count = *self.blockstate_map.get(&blockstate).unwrap_or(&0);
        self.blockstate_map
            .insert(blockstate.clone(), prev_blockstate_count + 1);

        self.layers
            .increment(blockstate.as_str(), block.global_pos.1);
    }

    fn count_chunk_section(&mut self, chunk_section: ChunkSection) {
        for block in chunk_section {
            self.count_blockstate(block);
        }
    }

    pub fn print_csv(self) {
        let mut blockstate_list: Vec<(String, u32)> = self
            .blockstate_map
            .iter()
            .map(|(block_id, count)| (block_id.clone(), *count))
            .collect();
        blockstate_list.sort_by(|(_, a), (_, b)| b.cmp(a));

        print!("Layer,");
        for (id, (blockstate, _)) in blockstate_list.iter().enumerate() {
            print!("{}", blockstate);
            if id < blockstate_list.len() - 1 {
                print!(",");
            }
        }
        println!();

        for layer in self.layers {
            print!("{:5},", layer.y);
            for (index, (blockstate, _)) in blockstate_list.iter().enumerate() {
                let layer_count = layer.get_count(blockstate);
                print!("{:8}", layer_count);
                if index < blockstate_list.len() - 1 {
                    print!(",");
                }
            }
            println!();
        }

        print!("Total,");
        for (index, (_, total_count)) in blockstate_list.iter().enumerate() {
            print!("{:8}", total_count);
            if index < blockstate_list.len() - 1 {
                print!(",");
            }
        }
        println!();
    }
}

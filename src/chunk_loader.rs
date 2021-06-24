use std::collections::{hash_map::Entry, HashMap, VecDeque};

use anvil_region::{
    position::{RegionChunkPosition, RegionPosition},
    provider::{FolderRegionProvider, RegionProvider},
};

use crate::{chunk::Chunk, chunk_section::CHUNK_SIZE};

const MAX_LOADED_CHUNKS: usize = 32;

pub struct ChunkLoader<'a> {
    loaded_chunks: HashMap<(i32, i32), Chunk>,
    recently_loaded_chunks: VecDeque<(i32, i32)>,
    region_provider: FolderRegionProvider<'a>,
}

impl<'a> ChunkLoader<'a> {
    pub fn new(region_folder: &'a str) -> Self {
        ChunkLoader {
            loaded_chunks: HashMap::new(),
            recently_loaded_chunks: VecDeque::new(),
            region_provider: FolderRegionProvider::new(region_folder),
        }
    }

    fn load_chunk(&mut self, coordinate: (i32, i32)) {
        if let Some(index) = self
            .recently_loaded_chunks
            .iter()
            .position(|&el| el == coordinate)
        {
            self.recently_loaded_chunks.remove(index);
            self.recently_loaded_chunks.push_back(coordinate);
        }
    }

    fn unload_chunks(&mut self) {
        if self.recently_loaded_chunks.len() >= MAX_LOADED_CHUNKS {
            for _ in 0..(self.recently_loaded_chunks.len() - MAX_LOADED_CHUNKS) {
                if let Some(least_recently_loaded_chunk) = self.recently_loaded_chunks.pop_front() {
                    self.loaded_chunks.remove(&least_recently_loaded_chunk);
                }
            }
        }
    }

    pub fn get_or_load(&mut self, chunk_x: i32, chunk_z: i32) -> &Chunk {
        self.load_chunk((chunk_x, chunk_z));
        self.unload_chunks();

        match self.loaded_chunks.entry((chunk_x, chunk_z)) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let chunk_pos = RegionChunkPosition::from_chunk_position(chunk_x, chunk_z);

                let mut region = self
                    .region_provider
                    .get_region(RegionPosition::from_chunk_position(chunk_x, chunk_z))
                    .expect("Could not load chunk file");

                let chunk_nbt = region.read_chunk(chunk_pos).expect("could not read chunk");
                let chunk = Chunk::from_nbt(&chunk_nbt);

                entry.insert(chunk)
            }
        }
    }

    pub fn get_blockstate_at(&mut self, x: i64, y: i32, z: i64) -> Option<&str> {
        let (chunk_x, chunk_z) = (x as i32 / CHUNK_SIZE as i32, z as i32 / CHUNK_SIZE as i32);
        let chunk = self.get_or_load(chunk_x, chunk_z);

        let section_index = y as i8 / CHUNK_SIZE as i8;
        let section = chunk.get_section(section_index)?;

        section.get_block_at(
            x as usize % CHUNK_SIZE,
            y as usize % CHUNK_SIZE,
            z as usize % CHUNK_SIZE,
        )
    }
}

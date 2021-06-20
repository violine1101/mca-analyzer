use std::io::Cursor;

use bitstream_io::{BitRead, BitReader, LittleEndian};
use nbt::CompoundTag;

use crate::palette::Palette;

pub const CHUNK_SIZE: usize = 16;
pub struct ChunkSection {
    blocks: [[[usize; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
}

impl ChunkSection {
    pub fn from_nbt(nbt: &CompoundTag, global_palette: &mut Palette) -> Self {
        // `Palette` nbt tag is implicitly empty if it doesn't exist
        let palette_nbt = nbt.get_compound_tag_vec("Palette").unwrap_or_default();
        let palette = Palette::from_nbt(palette_nbt);

        let blocks = if let Ok(block_state_array) = nbt.get_i64_vec("BlockStates") {
            get_blocks_in_chunk(block_state_array, palette, global_palette)
        } else {
            [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE]
        };

        Self { blocks }
    }
}

fn get_blocks_in_chunk(
    block_state_array: &[i64],
    chunk_section_palette: Palette,
    global_palette: &mut Palette,
) -> [[[usize; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE] {
    let mut result = [[[0; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

    let chunk_section_ids = get_block_ids_in_chunk(block_state_array, &chunk_section_palette);

    for (index, chunk_section_val) in chunk_section_ids.into_iter().enumerate() {
        if index >= CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
            break;
        }

        let chunk_section_id = chunk_section_val;
        let blockstate = chunk_section_palette
            .get_state(chunk_section_id)
            .expect("Chunk section palette index out of bounds");
        let global_id = global_palette.add(blockstate);

        let (x, y, z) = get_coords_from_array_pos(index);
        result[x][y][z] = global_id;
    }

    result
}

fn get_block_ids_in_chunk(block_state_array: &[i64], palette: &Palette) -> Vec<usize> {
    let width = palette.get_elem_bit_size();
    let mut result = Vec::with_capacity(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE);

    for &val in block_state_array {
        parse_blockstate_val(width, val).into_iter().for_each(|id| {
            result.push(id);
        });
    }

    result
}

fn parse_blockstate_val(width: u32, val: i64) -> Vec<usize> {
    let bytes = val.to_le_bytes();
    let cursor = Cursor::new(bytes);
    let mut reader = BitReader::endian(cursor, LittleEndian);

    let mut vec = Vec::with_capacity((64 / width) as usize);

    while let Ok(value) = reader.read::<u64>(width) {
        vec.push(value as usize);
    }

    vec
}

fn get_coords_from_array_pos(index: usize) -> (usize, usize, usize) {
    let x = index % CHUNK_SIZE;
    let z = (index / CHUNK_SIZE) % CHUNK_SIZE;
    let y = index / (CHUNK_SIZE * CHUNK_SIZE);

    (x, y, z)
}

pub struct ChunkSectionIterator {
    chunk_section: ChunkSection,
    index: usize,
}

pub struct ChunkSectionBlock {
    global_id: usize,
    pub chunk_pos: (usize, usize, usize),
}

impl ChunkSectionBlock {
    pub fn get_state<'a>(&self, global_palette: &'a Palette) -> &'a str {
        global_palette
            .get_state(self.global_id)
            .expect("Global palette is missing an entry")
    }
}

impl IntoIterator for ChunkSection {
    type Item = ChunkSectionBlock;

    type IntoIter = ChunkSectionIterator;

    fn into_iter(self) -> Self::IntoIter {
        ChunkSectionIterator {
            chunk_section: self,
            index: 0,
        }
    }
}

impl Iterator for ChunkSectionIterator {
    type Item = ChunkSectionBlock;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
            None
        } else {
            let (x, y, z) = get_coords_from_array_pos(self.index);
            self.index += 1;
            Some(ChunkSectionBlock {
                global_id: self.chunk_section.blocks[x][y][z],
                chunk_pos: (x, y, z),
            })
        }
    }
}

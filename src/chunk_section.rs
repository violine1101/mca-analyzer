use std::io::Cursor;

use bitstream_io::{BitRead, BitReader, LittleEndian};
use nbt::CompoundTag;

use crate::palette::Palette;

pub const CHUNK_SIZE: usize = 16;

struct BlocksArray {
    pub contents: [usize; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
}

const EMPTY_BLOCKS_ARRAY: BlocksArray = BlocksArray {
    contents: [0; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
};

impl BlocksArray {
    pub fn get(&self, x: usize, y: usize, z: usize) -> usize {
        let pos = y * CHUNK_SIZE * CHUNK_SIZE + z * CHUNK_SIZE + x;
        self.contents[pos]
    }
}

pub struct ChunkSection {
    blocks: BlocksArray,
    pub pos: (i32, i8, i32),
    palette: Palette,
}

impl ChunkSection {
    pub fn from_nbt(nbt: &CompoundTag, x: i32, z: i32) -> Option<Self> {
        // `Palette` nbt tag is implicitly empty if it doesn't exist
        let palette_nbt = nbt.get_compound_tag_vec("Palette").unwrap_or_default();
        let palette = Palette::from_nbt(palette_nbt);

        let blocks = if let Ok(block_state_array) = nbt.get_i64_vec("BlockStates") {
            get_blocks_in_chunk(block_state_array, &palette)
        } else {
            return None;
        };

        let y = nbt.get_i8("Y").ok()?;

        Some(Self {
            blocks,
            pos: (x, y, z),
            palette,
        })
    }

    pub fn get_block_at(&self, x: usize, y: usize, z: usize) -> Option<&str> {
        assert!(x < CHUNK_SIZE);
        assert!(y < CHUNK_SIZE);
        assert!(z < CHUNK_SIZE);

        let block_id = self.blocks.get(x, y, z);
        self.palette.get_state(block_id)
    }
}

fn get_blocks_in_chunk(block_state_array: &[i64], chunk_section_palette: &Palette) -> BlocksArray {
    let mut result = EMPTY_BLOCKS_ARRAY;

    let chunk_section_ids = get_block_ids_in_chunk(block_state_array, &chunk_section_palette);

    for (index, chunk_section_id) in chunk_section_ids.into_iter().enumerate() {
        if index >= CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
            break;
        }

        result.contents[index] = chunk_section_id;
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

pub struct ChunkSectionBlock {
    pub chunk_pos: (usize, usize, usize),
    pub global_pos: (i64, i32, i64),
    pub blockstate: String,
}

impl IntoIterator for ChunkSection {
    type Item = ChunkSectionBlock;

    type IntoIter = std::vec::IntoIter<ChunkSectionBlock>;

    fn into_iter(self) -> Self::IntoIter {
        let chunk_start = (
            self.pos.0 as i64 * CHUNK_SIZE as i64,
            self.pos.1 as i32 * CHUNK_SIZE as i32,
            self.pos.2 as i64 * CHUNK_SIZE as i64,
        );
        let palette = self.palette;

        let list: Vec<ChunkSectionBlock> = self
            .blocks
            .contents
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let chunk_pos = get_coords_from_array_pos(index);

                ChunkSectionBlock {
                    chunk_pos,
                    global_pos: (
                        chunk_start.0 + chunk_pos.0 as i64,
                        chunk_start.1 + chunk_pos.1 as i32,
                        chunk_start.2 + chunk_pos.2 as i64,
                    ),
                    blockstate: palette
                        .get_state(*id)
                        .expect("Blockstate is not in palette")
                        .to_string(),
                }
            })
            .collect();

        list.into_iter()
    }
}

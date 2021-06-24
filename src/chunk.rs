use std::collections::HashMap;

use nbt::CompoundTag;

use crate::chunk_section::ChunkSection;

pub struct Chunk {
    sections: HashMap<i8, ChunkSection>,
    pub x: i32,
    pub z: i32,
}

impl Chunk {
    pub fn from_nbt(nbt: &CompoundTag) -> Self {
        let level = nbt.get_compound_tag("Level").expect("Level doesn't exist");

        let x = level.get_i32("xPos").expect("xPos couldn't be parsed");
        let z = level.get_i32("zPos").expect("zPos couldn't be parsed");

        let sections = level
            .get_compound_tag_vec("Sections")
            .expect("Sections couldn't be parsed")
            .into_iter()
            .filter_map(|section_nbt| {
                let section = ChunkSection::from_nbt(section_nbt, x, z)?;
                Some((section.pos.1, section))
            })
            .collect();

        Chunk { sections, x, z }
    }
}

impl IntoIterator for Chunk {
    type Item = ChunkSection;

    type IntoIter = std::vec::IntoIter<ChunkSection>;

    fn into_iter(self) -> Self::IntoIter {
        let mut iter_list: Vec<(i8, ChunkSection)> = self.sections.into_iter().collect();
        iter_list.sort_by(|a, b| a.0.cmp(&b.0));

        let section_list: Vec<ChunkSection> =
            iter_list.into_iter().map(|(_, section)| section).collect();

        section_list.into_iter()
    }
}

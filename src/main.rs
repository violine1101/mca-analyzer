use std::convert::TryInto;
use std::io::Cursor;
use std::{collections::HashMap, path::Path};

use anvil_region::{
    position::RegionPosition,
    provider::{FolderRegionProvider, RegionProvider},
};
use bitstream_io::{BigEndian, BitRead, BitReader};
use clap::{App, Arg};
use nbt::CompoundTag;

fn parse_palette_entry(palette_entry: &CompoundTag) -> &str {
    palette_entry
        .get_str("Name")
        .expect("Couldn't get field Name for palette entry")
}

fn parse_blockstate(palette: &[&str], id: usize, blockstate_map: &mut HashMap<String, u32>) {
    let blockstate_name = *palette
        .get(id)
        .unwrap_or_else(|| panic!("blockstate index {} out of bounds", id));

    let prev_blockstate_count = *blockstate_map
        .get(&blockstate_name.to_string())
        .unwrap_or(&0);
    blockstate_map.insert(blockstate_name.to_string(), prev_blockstate_count + 1);
}

fn change_array_element_size(blockstates: &[i64], palette_length: usize) -> Vec<usize> {
    let palette_length: i32 = palette_length.try_into().unwrap();
    let width: u32 = f64::log2(palette_length.into()) as u32;

    let blockstate_bytes: Vec<u8> = blockstates
        .iter()
        .flat_map(|&val| Vec::from(val.to_le_bytes()))
        .collect();

    let cursor = Cursor::new(blockstate_bytes.as_slice());
    let mut reader = BitReader::endian(cursor, BigEndian);

    let mut result: Vec<usize> = vec![];

    for _x in 0..16 {
        for _y in 0..16 {
            for _z in 0..16 {
                let val = reader.read::<u64>(width).unwrap();
                result.push(val as usize);
            }
        }
    }

    result
}

fn iter_blocks_in_section(
    palette: &[&str],
    chunk_section: &CompoundTag,
    blockstate_map: &mut HashMap<String, u32>,
) {
    let block_states = chunk_section.get_i64_vec("BlockStates");

    if let Ok(block_states) = block_states {
        let indices = change_array_element_size(block_states, palette.len());

        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    let block_pos = y * 16 * 16 + z * 16 + x;
                    parse_blockstate(palette, indices[block_pos], blockstate_map);
                }
            }
        }
    }
}

fn parse_chunk_section(chunk_section: &CompoundTag, blockstate_map: &mut HashMap<String, u32>) {
    let palette: Vec<&str> = chunk_section
        .get_compound_tag_vec("Palette")
        .unwrap_or_default()
        .into_iter()
        .map(|entry| parse_palette_entry(entry))
        .collect();

    let result = chunk_section.get_i64_vec("BlockStates");

    if let Err(err) = result {
        match err {
            nbt::CompoundTagError::TagNotFound { .. } => {}
            nbt::CompoundTagError::TagWrongType { actual_tag, .. } => {
                panic!("WRONG TAG TYPE, CORRECT TYPE: {:?}", actual_tag)
            }
        }
    } else {
        iter_blocks_in_section(&palette, chunk_section, blockstate_map);
    }
}

fn main() {
    let matches = App::new("mca-analyzer")
        .version("0.1.0")
        .about("Analyze Minecraft's .mca region files")
        .arg(
            Arg::with_name("folder")
                .help("The region folder to be analyzed")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("An optional output file")
                .takes_value(true),
        )
        .get_matches();

    let input_path = if let Some(folder) = matches.value_of("folder") {
        let path = Path::new(folder);
        if !path.is_dir() {
            eprintln!("'{}' is not a folder!", folder);
            return;
        }
        path
    } else {
        eprintln!("No input folder has been specified.");
        return;
    };

    let _output_path = if let Some(output_file) = matches.value_of("output") {
        Some(Path::new(output_file))
    } else {
        None
    };

    let provider = FolderRegionProvider::new(input_path.to_str().unwrap());

    let region_pos = RegionPosition::from_chunk_position(0, 0);
    let region = provider
        .get_region(region_pos)
        .expect("Not a valid region file");

    let mut blockstate_map = HashMap::<String, u32>::new();

    for chunk in region {
        let level = chunk
            .get_compound_tag("Level")
            .expect("Level doesn't exist");
        let chunk_sections = level
            .get_compound_tag_vec("Sections")
            .expect("Sections couldn't be parsed");

        let chunk_x = level.get_i32("xPos").expect("xPos couldn't be parsed");
        let chunk_z = level.get_i32("zPos").expect("zPos couldn't be parsed");

        eprintln!("Analyzing chunk ({},{})", chunk_x, chunk_z);

        for section in chunk_sections.into_iter() {
            parse_chunk_section(section, &mut blockstate_map);
        }
    }

    let mut blockstate_list: Vec<(String, u32)> = blockstate_map.into_iter().collect();
    blockstate_list.sort_by(|(_, a), (_, b)| a.cmp(b));

    for (blockstate, count) in blockstate_list {
        println!("{:6} {}", count, blockstate);
    }
}

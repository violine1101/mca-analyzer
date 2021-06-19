use std::cmp::max;
use std::convert::TryInto;
use std::io::Cursor;
use std::{collections::HashMap, path::Path};

use anvil_region::{
    position::RegionPosition,
    provider::{FolderRegionProvider, RegionProvider},
};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use clap::{App, Arg};
use nbt::CompoundTag;

type Layer = HashMap<String, u32>;

fn parse_palette_entry(palette_entry: &CompoundTag) -> &str {
    palette_entry
        .get_str("Name")
        .expect("Couldn't get field Name for palette entry")
}

fn parse_blockstate(block: &str, blockstate_map: &mut HashMap<String, u32>, layer: &mut Layer) {
    let prev_blockstate_count = *blockstate_map.get(&block.to_string()).unwrap_or(&0);
    blockstate_map.insert(block.to_string(), prev_blockstate_count + 1);

    let prev_blockstate_count = *layer.get(&block.to_string()).unwrap_or(&0);
    layer.insert(block.to_string(), prev_blockstate_count + 1);
}

fn get_blocks_in_chunk_by_id<'a>(palette: &[&'a str], blockstates: &[i64]) -> Vec<&'a str> {
    let palette_length: i32 = palette.len().try_into().unwrap();
    let width: u32 = max(4, f64::log2(palette_length.into()).ceil() as u32);

    let mut result: Vec<&str> = vec![];

    for blockstate in blockstates {
        let bytes = blockstate.to_le_bytes();
        let cursor = Cursor::new(bytes);
        let mut reader = BitReader::endian(cursor, LittleEndian);

        while let Ok(value) = reader.read::<u64>(width) {
            if result.len() == 16 * 16 * 16 {
                return result;
            }

            let index = value as usize;
            if index >= palette.len() {
                panic!(
                    "Palette index out of bounds: {} (palette size {})",
                    index,
                    palette.len()
                );
            }
            result.push(palette[index]);
        }
    }

    result
}

fn iter_blocks_in_section(
    palette: &[&str],
    chunk_section: &CompoundTag,
    blockstate_map: &mut HashMap<String, u32>,
    section_layers: &mut [Layer],
) {
    let block_states = chunk_section.get_i64_vec("BlockStates");

    if let Ok(block_states) = block_states {
        let blocks = get_blocks_in_chunk_by_id(palette, block_states);

        for x in 0..16 {
            for z in 0..16 {
                for (y, layer) in section_layers.iter_mut().enumerate() {
                    let block_pos = y * 16 * 16 + z * 16 + x;
                    parse_blockstate(blocks[block_pos], blockstate_map, layer);
                }
            }
        }
    }
}

fn parse_chunk_section(
    chunk_section: &CompoundTag,
    blockstate_map: &mut HashMap<String, u32>,
    layers: &mut [Layer],
) {
    let section_y = chunk_section.get_i8("Y");

    // This section is empty
    if section_y.is_err() {
        return;
    }

    let section_y = section_y.unwrap();
    if section_y < 0 {
        return;
    }

    let section_y = section_y as usize;

    const SECTION_HEIGHT: usize = 16;
    let section_bottom_layer: usize = section_y * SECTION_HEIGHT;

    let section_y_range = section_bottom_layer..(section_bottom_layer + SECTION_HEIGHT);

    let section_layers = &mut layers[section_y_range];

    let mut palette: Vec<&str> = chunk_section
        .get_compound_tag_vec("Palette")
        .unwrap_or_default()
        .into_iter()
        .map(|entry| parse_palette_entry(entry))
        .collect();

    if palette.get(0) != Some(&"minecraft:air") {
        let mut old_palette = palette;
        palette = vec!["minecraft:air"];
        palette.append(&mut old_palette);
    }

    let result = chunk_section.get_i64_vec("BlockStates");

    if let Err(err) = result {
        match err {
            nbt::CompoundTagError::TagNotFound { .. } => {}
            nbt::CompoundTagError::TagWrongType { actual_tag, .. } => {
                panic!("WRONG TAG TYPE, CORRECT TYPE: {:?}", actual_tag)
            }
        }
    } else {
        iter_blocks_in_section(&palette, chunk_section, blockstate_map, section_layers);
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
    let mut layers = vec![Layer::new(); 256];

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
            parse_chunk_section(section, &mut blockstate_map, &mut layers);
        }
    }

    let mut blockstate_list: Vec<(String, u32)> = blockstate_map
        .iter()
        .map(|(block_id, count)| (block_id.clone(), *count))
        .collect();
    blockstate_list.sort_by(|(_, a), (_, b)| a.cmp(b));

    for (blockstate, count) in blockstate_list {
        println!("{:6} {}", count, blockstate);
    }

    println!("\nDiamonds:");

    for (y, layer) in layers.into_iter().enumerate() {
        let diamond_count = layer.get("minecraft:diamond_ore").cloned().unwrap_or(0);
        let deepslate_diamond_count = layer
            .get("minecraft:deepslate_diamond_ore")
            .cloned()
            .unwrap_or(0);
        let total_diamond_count = diamond_count + deepslate_diamond_count;

        let diamonds_string = format!("{:6}", total_diamond_count);

        println!("{:3} {}", y, diamonds_string);
    }

    let diamond_count = blockstate_map
        .get("minecraft:diamond_ore")
        .cloned()
        .unwrap_or(0);
    let deepslate_diamond_count = blockstate_map
        .get("minecraft:deepslate_diamond_ore")
        .cloned()
        .unwrap_or(0);
    let total_diamond_count = diamond_count + deepslate_diamond_count;

    let diamonds_string = format!("{:6}", total_diamond_count);

    println!("\ntotal: {}", diamonds_string);
}

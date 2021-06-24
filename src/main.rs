use std::{collections::HashMap, path::Path};

use anvil_region::position::RegionChunkPosition;
use anvil_region::{
    position::RegionPosition,
    provider::{FolderRegionProvider, RegionProvider},
};
use chunk_section::{ChunkSection, ChunkSectionBlock};
use clap::{App, Arg};
use layers::Layers;

use crate::area::Area;
use crate::chunk::Chunk;

mod area;
mod chunk;
mod chunk_section;
mod layers;
mod palette;

fn count_blockstate(
    block: ChunkSectionBlock,
    blockstate_map: &mut HashMap<String, u32>,
    layers: &mut Layers,
) {
    let blockstate = block.blockstate;

    let prev_blockstate_count = *blockstate_map.get(&blockstate).unwrap_or(&0);
    blockstate_map.insert(blockstate.clone(), prev_blockstate_count + 1);

    layers.increment(blockstate.as_str(), block.global_pos.1);
}

fn count_chunk_section(
    chunk_section: ChunkSection,
    blockstate_map: &mut HashMap<String, u32>,
    layers: &mut Layers,
) {
    for block in chunk_section {
        count_blockstate(block, blockstate_map, layers);
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

    let region_provider = FolderRegionProvider::new(input_path.to_str().unwrap());

    let mut blockstate_map = HashMap::<String, u32>::new();
    let mut layers = Layers::new();

    let area = Area::new(0, 256, 0, 256);

    for (chunk_x, chunk_z) in area {
        let chunk_pos = RegionChunkPosition::from_chunk_position(chunk_x, chunk_z);
        let mut region = region_provider
            .get_region(RegionPosition::from_chunk_position(chunk_x, chunk_z))
            .expect("Could not load chunk file");

        let chunk_nbt = region.read_chunk(chunk_pos).expect("could not read chunk");

        let chunk = Chunk::from_nbt(&chunk_nbt);

        eprintln!("Analyzing chunk ({},{})", chunk_x, chunk_z);

        for section in chunk {
            count_chunk_section(section, &mut blockstate_map, &mut layers);
        }
    }

    let mut blockstate_list: Vec<(String, u32)> = blockstate_map
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

    for layer in layers {
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

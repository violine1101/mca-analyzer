use std::{collections::HashMap, path::Path};

use anvil_region::position::RegionChunkPosition;
use anvil_region::{
    position::RegionPosition,
    provider::{FolderRegionProvider, RegionProvider},
};
use chunk_section::ChunkSection;
use clap::{App, Arg};
use palette::Palette;

use crate::full_chunk::Chunk;

mod chunk_section;
mod full_chunk;
mod palette;

type Layer = HashMap<String, u32>;

fn count_blockstate(block: &str, blockstate_map: &mut HashMap<String, u32>, layer: &mut Layer) {
    let prev_blockstate_count = *blockstate_map.get(&block.to_string()).unwrap_or(&0);
    blockstate_map.insert(block.to_string(), prev_blockstate_count + 1);

    let prev_blockstate_count = *layer.get(&block.to_string()).unwrap_or(&0);
    layer.insert(block.to_string(), prev_blockstate_count + 1);
}

fn count_chunk_section(
    chunk_section: ChunkSection,
    blockstate_map: &mut HashMap<String, u32>,
    layers: &mut [Layer],
    global_palette: &mut Palette,
) {
    let section_y = if let Some(y) = chunk_section.y {
        y as usize
    } else {
        return;
    };

    const SECTION_HEIGHT: usize = 16;
    let section_bottom_layer: usize = section_y * SECTION_HEIGHT;

    let section_y_range = section_bottom_layer..(section_bottom_layer + SECTION_HEIGHT);

    let section_layers = &mut layers[section_y_range];

    for block in chunk_section {
        let blockstate = block.get_state(&global_palette);
        count_blockstate(
            blockstate,
            blockstate_map,
            &mut section_layers[block.chunk_pos.1],
        );
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
    let mut layers = vec![Layer::new(); 256];

    let mut global_palette = Palette::new();

    for chunk_x in 0..128 {
        for chunk_z in 0..128 {
            let chunk_pos = RegionChunkPosition::from_chunk_position(chunk_x, chunk_z);
            let mut region = region_provider
                .get_region(RegionPosition::from_chunk_position(chunk_x, chunk_z))
                .expect("Could not load chunk file");

            let chunk_nbt = region.read_chunk(chunk_pos).expect("could not read chunk");

            let chunk = Chunk::from_nbt(&chunk_nbt, &mut global_palette);

            eprintln!("Analyzing chunk ({},{})", chunk_x, chunk_z);

            for section in chunk {
                count_chunk_section(
                    section,
                    &mut blockstate_map,
                    &mut layers,
                    &mut global_palette,
                );
            }
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

    for (y, layer) in layers.iter().enumerate() {
        print!("{},", y);
        for (index, (blockstate, _)) in blockstate_list.iter().enumerate() {
            let layer_count = layer.get(blockstate).unwrap_or(&0);
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

use std::{
    cmp::min,
    collections::{HashMap, HashSet},
};

use image::{ImageBuffer, Rgb, RgbImage};
use itertools::Itertools;

use crate::{area::Area, chunk::Chunk, chunk_loader::ChunkLoader};

struct Vein {
    blocks: HashSet<(i64, i32, i64)>,

    /// Smallest coordinate in the vein.
    /// Priority: x y z
    location: (i64, i32, i64),
}

const MAX_VEIN_SIZE: usize = 16;

const DIAMOND_ORES: &[&str] = &["minecraft:diamond_ore", "minecraft:deepslate_diamond_ore"];

pub struct DiamondVeinAnalyzer<'a> {
    chunk_loader: ChunkLoader<'a>,

    found_veins: HashSet<(i64, i32, i64)>,

    /// (size, height) -> count
    vein_sizes: HashMap<(u8, i16), u32>,

    /// diamond count -> # chunks with that diamond count
    diamonds_per_chunk: HashMap<u8, u32>,

    area: Area,
}

impl<'a> DiamondVeinAnalyzer<'a> {
    pub fn new(path: &'a str, area: Area) -> Self {
        let chunk_loader = ChunkLoader::new(path, Some(0..4));

        DiamondVeinAnalyzer {
            chunk_loader,
            found_veins: HashSet::new(),
            vein_sizes: HashMap::new(),
            diamonds_per_chunk: HashMap::new(),
            area,
        }
    }

    pub fn analyze(&mut self) {
        for (chunk_x, chunk_z) in self.area {
            let chunk = self.chunk_loader.get_or_load(chunk_x, chunk_z).clone();

            eprintln!("Analyzing chunk ({},{})", chunk_x, chunk_z);

            let diamonds_in_chunk = self.analyze_chunk(chunk);

            let chunks_with_this_many_diamonds = self
                .diamonds_per_chunk
                .entry(diamonds_in_chunk)
                .or_insert(0);

            *chunks_with_this_many_diamonds += 1;
        }
    }

    /// returns: number of diamonds in chunk
    fn analyze_chunk(&mut self, chunk: Chunk) -> u8 {
        let mut diamond_count: u8 = 0;

        for section in chunk {
            for block in section {
                if DIAMOND_ORES.contains(&block.blockstate.as_str()) {
                    let (x, y, z) = block.global_pos;

                    let vein = Vein {
                        blocks: HashSet::new(),
                        location: (x, y, z),
                    };

                    if let Some(vein) = self.explore_vein(vein, x, y, z) {
                        self.found_veins.insert(vein.location);

                        let count = self
                            .vein_sizes
                            .entry((vein.blocks.len() as u8, vein.location.1 as i16))
                            .or_insert(0);

                        *count += 1;
                    }

                    diamond_count += 1;
                }
            }
        }

        diamond_count
    }

    fn explore_vein(&mut self, mut vein: Vein, x: i64, y: i32, z: i64) -> Option<Vein> {
        if vein.blocks.len() >= MAX_VEIN_SIZE || self.found_veins.contains(&(x, y, z)) {
            return None;
        }

        if vein.blocks.contains(&(x, y, z)) {
            return Some(vein);
        }

        if let Some(block) = self.chunk_loader.get_blockstate_at(x, y, z) {
            if DIAMOND_ORES.contains(&block) {
                vein.blocks.insert((x, y, z));
                vein.location = min_coord(vein.location, (x, y, z));

                for rx in -1..=1 {
                    for ry in -1..=1 {
                        for rz in -1..=1 {
                            vein = self.explore_vein(vein, x + rx, y + ry, z + rz)?;
                        }
                    }
                }
            }
        }

        if vein.blocks.is_empty() {
            None
        } else {
            Some(vein)
        }
    }

    pub fn print_csv(&self) {
        eprintln!("Printing number of diamonds / chunk");

        println!("Number of diamonds,Chunks");
        let mut diamonds_per_chunk: Vec<_> = self.diamonds_per_chunk.iter().collect();
        diamonds_per_chunk.sort_unstable();

        for (diamonds, chunks) in diamonds_per_chunk {
            println!("{:8},{:8}", diamonds, chunks);
        }
        println!();

        eprintln!("Preparing to print diamond vein table...");

        let mut sizes: Vec<u8> = self
            .vein_sizes
            .keys()
            .map(|(vein_size, _height)| *vein_size)
            .collect();
        sizes.sort_unstable();
        sizes.dedup();

        let mut heights: Vec<i16> = self
            .vein_sizes
            .keys()
            .map(|(_vein_size, height)| *height)
            .collect();
        heights.sort_unstable();
        heights.dedup();

        eprintln!("Printing diamond vein table...");

        print!("Veins");
        for size in sizes.iter() {
            print!(",{:8}", size)
        }
        println!();

        for height in heights {
            print!("{:5}", height);

            for &size in sizes.iter() {
                let value = self.vein_sizes.get(&(size, height)).cloned().unwrap_or(0);
                print!(",{:8}", value);
            }

            println!();
        }

        eprintln!("Done printing CSV!");
    }

    pub fn print_img(&self, path: &str) {
        eprintln!("Preparing to print image...");

        let img_area = self.area.to_vis_coords();

        let canvas_width = img_area.block_width_x();
        let canvas_height = img_area.block_width_z();

        let mut veins: Vec<(u32, u32)> = self
            .found_veins
            .iter()
            .map(|(x, _y, z)| (*x as u32, *z as u32))
            .collect();
        veins.sort_unstable();

        let veins: HashMap<(u32, u32), usize> = veins
            .into_iter()
            .dedup_with_count()
            .map(|(count, (x, z))| ((x, z), count))
            .collect();

        eprintln!("Printing image...");

        let img: RgbImage = ImageBuffer::from_fn(canvas_width, canvas_height, |x, y| {
            let y = canvas_height - y - 1;

            if let Some(&count) = veins.get(&(x, y)) {
                let brightness = min(count as u8 * 32, 127);
                Rgb([0, 0, brightness])
            } else {
                Rgb([255, 255, 255])
            }
        });

        eprintln!("Saving image...");

        img.save(path).unwrap();

        eprintln!("Done printing image!");
    }
}

fn min_coord(a: (i64, i32, i64), b: (i64, i32, i64)) -> (i64, i32, i64) {
    if a.0 < b.0 || a.1 < b.1 || a.2 < b.2 {
        a
    } else {
        b
    }
}

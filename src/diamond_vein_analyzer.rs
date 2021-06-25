use std::collections::{HashMap, HashSet};

use image::{ImageBuffer, Rgb, RgbImage};

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

    /// size -> count
    vein_count_by_size: HashMap<u8, u32>,

    /// height -> count
    vein_count_by_height: HashMap<i16, u32>,

    /// diamond count -> # chunks with that diamond count
    diamonds_per_chunk: HashMap<u8, u32>,

    diamond_img: RgbImage,

    area: Area,
}

impl<'a> DiamondVeinAnalyzer<'a> {
    pub fn new(path: &'a str, area: Area) -> Self {
        let chunk_loader = ChunkLoader::new(path, Some(0..4));

        let diamond_img: RgbImage = ImageBuffer::from_pixel(
            area.chunk_width_x(),
            area.chunk_width_z(),
            Rgb([255, 255, 255]),
        );

        DiamondVeinAnalyzer {
            chunk_loader,
            found_veins: HashSet::new(),
            vein_count_by_size: HashMap::new(),
            vein_count_by_height: HashMap::new(),
            diamonds_per_chunk: HashMap::new(),
            diamond_img,
            area,
        }
    }

    pub fn clean_found_veins(&mut self, (x, z): (i64, i64)) {
        self.found_veins = self
            .found_veins
            .iter()
            .filter(|&(lx, _, lz)| *lx < x && *lz < z)
            .cloned()
            .collect();
    }

    pub fn analyze(&mut self) {
        for (chunk_x, chunk_z) in self.area {
            let chunk = self.chunk_loader.get_or_load(chunk_x, chunk_z).clone();
            let chunk_pos = chunk.get_global_pos();

            eprintln!(
                "Analyzing chunk ({},{}). [fv {}, cs {}, ch {}, dc {}]",
                chunk_x,
                chunk_z,
                self.found_veins.len(),
                self.vein_count_by_size.len(),
                self.vein_count_by_height.len(),
                self.diamonds_per_chunk.len(),
            );

            let diamonds_in_chunk = self.analyze_chunk(chunk);

            let chunks_with_this_many_diamonds = self
                .diamonds_per_chunk
                .entry(diamonds_in_chunk)
                .or_insert(0);

            *chunks_with_this_many_diamonds += 1;

            self.update_img(chunk_x, chunk_z, diamonds_in_chunk);

            self.clean_found_veins(chunk_pos);
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
                        vein.blocks.iter().for_each(|&pos| {
                            self.found_veins.insert(pos);
                        });

                        *self
                            .vein_count_by_size
                            .entry(vein.blocks.len() as u8)
                            .or_insert(0) += 1;

                        *self
                            .vein_count_by_height
                            .entry(vein.location.1 as i16)
                            .or_insert(0) += 1;
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
        let mut diamonds_per_chunk: Vec<(&u8, &u32)> = self.diamonds_per_chunk.iter().collect();
        diamonds_per_chunk.sort_unstable();

        for (diamonds, chunks) in diamonds_per_chunk {
            println!("{:8},{:8}", diamonds, chunks);
        }
        println!();

        eprintln!("Preparing to print diamond vein tables...");

        let mut sizes: Vec<(&u8, &u32)> = self.vein_count_by_size.iter().collect();
        sizes.sort_unstable();
        sizes.dedup();

        let mut heights: Vec<(&i16, &u32)> = self.vein_count_by_height.iter().collect();
        heights.sort_unstable();
        heights.dedup();

        eprintln!("Printing diamond vein size table...");

        println!("Vein Size,Vein Count");
        for (size, count) in sizes {
            println!("{:8},{:8}", size, count)
        }
        println!();

        eprintln!("Printing diamond vein height table...");

        println!("Vein Height,Vein Count");
        for (height, count) in heights {
            println!("{:8},{:8}", height, count);
        }

        eprintln!("Done printing CSV!");
    }

    pub fn update_img(&mut self, chunk_x: i32, chunk_z: i32, diamond_count: u8) {
        let (x, y) = self.area.get_positive_coords(chunk_x, chunk_z);
        let y = self.area.chunk_width_z() - y - 1;

        let brightness = 255u32.saturating_sub(diamond_count as u32 * 16) as u8;
        let pixel = Rgb([0, 0, brightness]);

        self.diamond_img.put_pixel(x, y, pixel);
    }

    pub fn print_img(&self, path: &str) {
        eprintln!("Saving image...");

        self.diamond_img.save(path).unwrap();

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

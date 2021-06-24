use std::ops::Range;

use crate::chunk_section::CHUNK_SIZE;

#[derive(Debug, Clone, Copy)]
pub struct Area {
    x_range: (i32, i32),
    z_range: (i32, i32),
}

impl Area {
    pub fn new(min_x: i32, max_x: i32, min_z: i32, max_z: i32) -> Self {
        Area {
            x_range: (min_x, max_x),
            z_range: (min_z, max_z),
        }
    }

    pub fn to_vis_coords(&self) -> Area {
        Area {
            x_range: (0, self.x_range.1 - self.x_range.0),
            z_range: (0, self.z_range.1 - self.z_range.0),
        }
    }

    pub fn block_width_x(&self) -> u32 {
        let area = self.to_vis_coords();
        area.x_range.1 as u32 * CHUNK_SIZE as u32
    }

    pub fn block_width_z(&self) -> u32 {
        let area = self.to_vis_coords();
        area.z_range.1 as u32 * CHUNK_SIZE as u32
    }
}

impl IntoIterator for Area {
    type Item = (i32, i32);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        rangeify_tuple(self.z_range)
            .flat_map(move |z| rangeify_tuple(self.x_range).map(move |x| (x, z)))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

fn rangeify_tuple((start, end): (i32, i32)) -> Range<i32> {
    start..end
}

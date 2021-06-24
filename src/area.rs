use std::ops::Range;

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

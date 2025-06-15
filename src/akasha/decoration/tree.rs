use rand::Rng;

use crate::akasha::decoration::{Decoration, WorldPos};

pub struct Tree {
    tree_x: i32,
    tree_y: i32,
    tree_z: i32,
    tree_height: u32,
}

impl Decoration for Tree {
    type Locus = WorldPos;
    /*fn decorate<'a>(self, neighborhood: &'a mut crate::world::Neighborhood) {
        let Tree {
            tree_x,
            tree_y,
            tree_z,
            tree_height,
        } = self;

        let tree_height = tree_height as i32;

        for dy in 0..(tree_height / 2) {
            for dx in -2..=2 {
                for dz in -2..=2 {
                    if (dx as i32).abs() + (dz as i32).abs() <= 2 {
                        neighborhood.set_block(
                            (tree_x + dx) as i32,
                            (tree_y + tree_height - dy) as i32,
                            (tree_z + dz) as i32,
                            6, // Assuming block ID 6 is a leaf
                        );
                    }
                }
            }
        }
        for dy in 0..2 {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    if (dx as i32).abs() + (dz as i32).abs() <= 2 {
                        neighborhood.set_block(
                            (tree_x + dx) as i32,
                            (tree_y + tree_height + dy + 1) as i32,
                            (tree_z + dz) as i32,
                            6, // Assuming block ID 6 is a leaf
                        );
                    }
                }
            }
        }
        for dy in 0..tree_height {
            neighborhood.set_block(
                tree_x as i32,
                (tree_y + dy) as i32,
                tree_z as i32,
                5, // Assuming block ID 5 is a log
            );
        }
    }*/

    fn from_rng<R: rand::Rng>(rng: &mut R, locus: &Self::Locus) -> Self
    where
        Self: Sized,
    {
        Tree {
            tree_x: locus.x,
            tree_y: locus.y,
            tree_z: locus.z,
            tree_height: rng.random_range(2..=8), // Random height between 4 and 8
        }
    }
}

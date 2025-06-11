use std::hash::Hash;

use crate::world::Neighborhood;

use super::locus_into_rng;

pub mod tree;

#[derive(Hash)]
pub struct WorldPos {
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub chunk_z: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub trait Decoration {
    type Locus: Hash;

    fn decorate<'a>(self, neighborhood: &'a mut Neighborhood);

    fn from_rng<R: rand::Rng>(rng: &mut R, locus: &Self::Locus) -> Self
    where
        Self: Sized;

    fn from_locus(locus: Self::Locus) -> Self
    where
        Self: Sized,
    {
        let mut rng = locus_into_rng(&locus);
        Self::from_rng(&mut rng, &locus)
    }
}

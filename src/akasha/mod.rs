use std::hash::{Hash, Hasher};

use rand::SeedableRng;

pub mod decoration;

fn locus_into_seed<T: Hash>(locus: T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    locus.hash(&mut hasher);
    hasher.finish()
}

fn locus_into_rng<T: Hash>(locus: &T) -> rand::rngs::StdRng {
    let seed = locus_into_seed(locus);
    rand::rngs::StdRng::seed_from_u64(seed)
}

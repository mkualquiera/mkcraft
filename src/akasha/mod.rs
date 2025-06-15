use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, RwLock},
};

use rand::SeedableRng;
use simdnoise::NoiseBuilder;

use crate::world::CHUNK_SIZE_X;

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

pub struct ChunkNoises {
    pub noise: Vec<f32>,
    pub noise_mountains: Vec<f32>,
    pub dirt_noise: Vec<f32>,
    pub variance: Vec<f32>,
}

impl ChunkNoises {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let (noise, _, _) = NoiseBuilder::fbm_2d_offset(
            (x * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
            (z * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
        )
        .with_freq(0.0001)
        .with_octaves(8)
        .with_gain(2.2)
        .with_seed(42)
        .with_lacunarity(2.0)
        .generate();

        let (noise_mountains, _, _) = NoiseBuilder::ridge_2d_offset(
            (x * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
            (z * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
        )
        .with_freq(0.01 / 64000.0)
        .with_octaves(12)
        .with_gain(2.3)
        .with_seed(42)
        .with_lacunarity(2.2)
        .generate();

        let (dirt_noise, min, max) = NoiseBuilder::fbm_2d_offset(
            (x * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
            (z * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
        )
        .with_freq(0.0001)
        .with_octaves(1)
        .with_gain(2.0)
        .with_seed(44)
        .with_lacunarity(2.0)
        .generate();

        let (variance, _, _) = NoiseBuilder::fbm_2d_offset(
            (x * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
            (z * CHUNK_SIZE_X) as f32,
            CHUNK_SIZE_X as usize,
        )
        .with_freq(1.0 / 2000.0)
        .with_octaves(1)
        .with_gain(1.0)
        .with_seed(43)
        .with_lacunarity(1.0)
        .generate();

        ChunkNoises {
            noise,
            noise_mountains,
            dirt_noise,
            variance,
        }
    }
}

pub struct AkashaChunk {
    pub chunk_noises: ChunkNoises,
}

pub struct Akasha {
    pub chunks: Arc<RwLock<HashMap<(i32, i32, i32), Arc<RwLock<AkashaChunk>>>>>,
}

impl Akasha {
    pub fn new() -> Self {
        Akasha {
            chunks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn ensure_chunk(
        akasha: &Arc<Akasha>,
        x: i32,
        y: i32,
        z: i32,
    ) -> Arc<RwLock<AkashaChunk>> {
        let mut chunks = akasha.chunks.write().unwrap();
        if let Some(chunk) = chunks.get(&(x, y, z)) {
            return chunk.clone();
        }

        let chunk = Arc::new(RwLock::new(AkashaChunk {
            chunk_noises: ChunkNoises::new(x, y, z),
        }));
        chunks.insert((x, y, z), chunk.clone());
        chunk
    }
}

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, RwLock},
};

use rand::SeedableRng;
use simdnoise::NoiseBuilder;

use crate::{akasha::decoration::tree::Tree, world::CHUNK_SIZE_X};

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

    pub target_height: Vec<i32>,
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

        let mut target_height =
            Vec::with_capacity((CHUNK_SIZE_X * CHUNK_SIZE_X) as usize);
        for i in 0..CHUNK_SIZE_X * CHUNK_SIZE_X {
            let i = i as usize;
            let base_noise = noise[i];
            let mountains_noise = -noise_mountains[i];
            let variance_noise = variance[i];
            let normalized_variance = ((variance_noise / 0.02) + 1.0) / 2.0;
            let target_height_value = (mountains_noise * normalized_variance
                + base_noise * (1.0 - normalized_variance))
                as i32;
            target_height.push(target_height_value);
        }

        ChunkNoises {
            noise,
            noise_mountains,
            dirt_noise,
            variance,
            target_height,
        }
    }
}

pub struct ChunkDecorations {
    pub trees: Vec<Tree>,
}

pub struct AkashaChunk {
    pub noises: ChunkNoises,
    pub decorations: ChunkDecorations,
}

impl AkashaChunk {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        let noises = ChunkNoises::new(x, y, z);
        let decorations = ChunkDecorations { trees: Vec::new() };
        AkashaChunk {
            noises,
            decorations,
        }
    }
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
        {
            let chunks = akasha.chunks.read().unwrap();
            if let Some(chunk) = chunks.get(&(x, y, z)) {
                return chunk.clone();
            }
        }

        let mut chunks = akasha.chunks.write().unwrap();
        let chunk = Arc::new(RwLock::new(AkashaChunk::new(x, y, z)));
        chunks.insert((x, y, z), chunk.clone());
        chunk
    }
}

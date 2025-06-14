use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, RwLock, RwLockWriteGuard},
};

use rand::{
    Rng, SeedableRng,
};
use simdnoise::NoiseBuilder;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};


pub const CHUNK_SIZE_X: i32 = 32;
pub const CHUNK_SIZE: i32 = CHUNK_SIZE_X * CHUNK_SIZE_X * CHUNK_SIZE_X; // CHUNK_SIZE_XxCHUNK_SIZE_XxCHUNK_SIZE_X = 4096 blocks per chunk

struct ChunkData {
    pub block_ids: [u8; CHUNK_SIZE as usize],
    pub height_map: [Option<i32>; (CHUNK_SIZE_X * CHUNK_SIZE_X) as usize],
}

struct ChunkNoises {
    pub noise: Vec<f32>,
    pub noise_mountains: Vec<f32>,
    pub dirt_noise: Vec<f32>,
    pub variance: Vec<f32>,
    pub rng: rand::rngs::StdRng,
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

        let mut hasher = DefaultHasher::new();
        (x, y, z).hash(&mut hasher);
        let seed = hasher.finish() as u64;

        ChunkNoises {
            noise,
            noise_mountains,
            dirt_noise,
            variance,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }
}

pub struct Neighborhood<'a> {
    //pub data: [RwLockWriteGuard<'a, ChunkState>; 3 * 3 * 3],
    pub data: Vec<RwLockWriteGuard<'a, ChunkState>>,
    pub size: usize,
    pub offset: (i32, i32, i32),
}

impl<'a> Neighborhood<'a> {
    pub fn get_chunk(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
    ) -> &mut RwLockWriteGuard<'a, ChunkState> {
        let (ox, oy, oz) = self.offset;
        let dx = x.div_euclid(CHUNK_SIZE_X) + ox;
        let dy = y.div_euclid(CHUNK_SIZE_X) + oy;
        let dz = z.div_euclid(CHUNK_SIZE_X) + oz;
        let size = self.size as i32;
        let offset = (self.size / 2) as i32;
        let arr_index = ((dx + offset) * size * size
            + (dy + offset) * size
            + (dz + offset)) as usize;
        if arr_index < self.data.len() {
            &mut self.data[arr_index]
        } else {
            panic!(
                "Index {} out of bounds for neighborhood data array. x={}, y={}, z={}; dx={}, dy={}, dz={}",
                arr_index, x, y, z, dx, dy, dz
            );
        }
    }

    pub fn get_chunk_immutable(
        &self,
        x: i32,
        y: i32,
        z: i32,
    ) -> &RwLockWriteGuard<'a, ChunkState> {
        let (ox, oy, oz) = self.offset;
        let dx = x.div_euclid(CHUNK_SIZE_X) + ox;
        let dy = y.div_euclid(CHUNK_SIZE_X) + oy;
        let dz = z.div_euclid(CHUNK_SIZE_X) + oz;
        let size = self.size as i32;
        let offset = (self.size / 2) as i32;
        let arr_index = ((dx + offset) * size * size
            + (dy + offset) * size
            + (dz + offset)) as usize;
        if arr_index < self.data.len() {
            &self.data[arr_index]
        } else {
            panic!("Index out of bounds for neighborhood data array");
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block_id: u8) {
        let chunk = self.get_chunk(x, y, z);
        chunk.set_block(
            x.rem_euclid(CHUNK_SIZE_X) as usize,
            y.rem_euclid(CHUNK_SIZE_X) as usize,
            z.rem_euclid(CHUNK_SIZE_X) as usize,
            block_id,
        );
    }
    pub async fn get_block(&mut self, x: i32, y: i32, z: i32) -> u8 {
        let chunk = self.get_chunk(x, y, z);
        chunk.get_block(
            x.rem_euclid(CHUNK_SIZE_X) as usize,
            y.rem_euclid(CHUNK_SIZE_X) as usize,
            z.rem_euclid(CHUNK_SIZE_X) as usize,
        )
    }
}

impl ChunkData {
    pub fn new(basis_x: i32, basis_y: i32, basis_z: i32, noises: &ChunkNoises) -> Self {
        let mut block_ids = [0; CHUNK_SIZE as usize];
        let mut height_map = [None; (CHUNK_SIZE_X * CHUNK_SIZE_X) as usize];

        let noise = &noises.noise;
        let noise_mountains = &noises.noise_mountains;
        let variance = &noises.variance;

        // do some stuff for now using sine to generate some blocks
        for x in 0..CHUNK_SIZE_X {
            for y in 0..CHUNK_SIZE_X {
                for z in 0..CHUNK_SIZE_X {
                    let index = x + y * CHUNK_SIZE_X + z * CHUNK_SIZE_X * CHUNK_SIZE_X;

                    let global_x = basis_x * CHUNK_SIZE_X + x as i32;
                    let global_y = basis_y * CHUNK_SIZE_X + y as i32;
                    let global_z = basis_z * CHUNK_SIZE_X + z as i32;

                    //let target_height =
                    //    (global_x as f64 * 0.1 + global_z as f64 * 0.1).sin() * 5.0 + 5.0;

                    let base_noise = noise[(x + z * CHUNK_SIZE_X) as usize];
                    let mountains_noise =
                        -noise_mountains[(x + z * CHUNK_SIZE_X) as usize];
                    let variance_noise = variance[(x + z * CHUNK_SIZE_X) as usize];
                    let normalized_variance = ((variance_noise / 0.02) + 1.0) / 2.0;

                    let target_height = (mountains_noise * normalized_variance
                        + base_noise * (1.0 - normalized_variance))
                        as i32;

                    let dirt_height = target_height + 2;
                    let grass_height = dirt_height + 1;

                    block_ids[index as usize] = 0;
                    if global_y <= 0 {
                        block_ids[index as usize] = 4;
                    }
                    if global_y == grass_height as i32 {
                        if global_y >= 0 {
                            block_ids[index as usize] = 3;
                            if global_y > 0 {
                                height_map[(x + z * CHUNK_SIZE_X) as usize] =
                                    Some(y as i32);
                            }
                        } else {
                            block_ids[index as usize] = 2; // Dirt
                        }
                    }
                    if global_y <= dirt_height as i32 {
                        block_ids[index as usize] = 2;
                    }
                    if global_y <= target_height as i32 {
                        block_ids[index as usize] = 1;
                    }
                }
            }
        }

        ChunkData {
            block_ids,
            height_map,
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_id: u8) {
        let usize_c = CHUNK_SIZE_X as usize;
        let index = x + y * usize_c + z * usize_c * usize_c;
        if index < (CHUNK_SIZE as usize) {
            self.block_ids[index] = block_id;
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u8 {
        let usize_c = CHUNK_SIZE_X as usize;
        let index = x + y * usize_c + z * usize_c * usize_c;
        if index < (CHUNK_SIZE as usize) {
            self.block_ids[index]
        } else {
            0 // Return air or empty block
        }
    }
}

pub struct ChunkState {
    pub data: Option<ChunkData>,
    pub noises: Option<ChunkNoises>,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkState {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            data: None,
            noises: None,
            x,
            y,
            z,
        }
    }

    pub fn ensure_noised(&mut self) {
        if self.noises.is_none() {
            self.noises = Some(ChunkNoises::new(self.x, self.y, self.z));
        }
    }

    pub fn ensure_formed(&mut self) {
        if self.data.is_none() {
            self.ensure_noised();
            let noises = self.noises.as_ref().expect("Noises must be initialized");
            self.data = Some(ChunkData::new(self.x, self.y, self.z, noises));
        }
    }

    pub fn is_formed(&self) -> bool {
        self.data.is_some()
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_id: u8) {
        self.ensure_formed();
        if let Some(data) = &mut self.data {
            data.set_block(x, y, z, block_id);
        } else {
            panic!("Chunk data must be initialized before setting a block");
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u8 {
        if let Some(data) = &self.data {
            data.get_block(x, y, z)
        } else {
            panic!("Chunk data must be initialized before getting a block");
        }
    }
}

pub struct ChunkUpdateMessage {
    pub world: Arc<World>,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub struct World {
    pub chunks: Arc<RwLock<HashMap<(i32, i32, i32), Arc<RwLock<ChunkState>>>>>,
    pub chunk_update_listeners: Vec<UnboundedSender<ChunkUpdateMessage>>,
}

impl World {
    pub fn new() -> Self {
        let mut colors = HashMap::new();
        // Set random colors for blocks
        let mut rng = rand::rng();
        for i in 1..=255 {
            let r = rng.random_range(0.0..1.0);
            let g = rng.random_range(0.0..1.0);
            let b = rng.random_range(0.0..1.0);
            colors.insert(i, [r, g, b, 1.0]); // RGBA
        }
        World {
            chunks: Arc::new(RwLock::new(HashMap::new())),
            chunk_update_listeners: Vec::new(),
        }
    }

    pub fn ensure_chunk(
        world: &Arc<World>,
        x: i32,
        y: i32,
        z: i32,
    ) -> Arc<RwLock<ChunkState>> {
        //world.chunks.entry((x, y, z)).or_insert_with(|| {
        //    let mut state = ChunkState::new(x, y, z);
        //    state.ensure_decorated();
        //    state
        //})

        // Lock to ensure the chunk exists
        let mut chunks_read = world.chunks.write().unwrap();

        let mut chunk_arc_init: Option<Arc<RwLock<ChunkState>>> = None;
        if let Some(chunk) = chunks_read.get(&(x, y, z)) {
            chunk_arc_init = Some(Arc::clone(chunk));
        } /* else {
        // Otherwise, create a new chunk state
        chunk_arc = Arc::new(RwLock::new(ChunkState::new(x, y, z)));
        chunks.insert((x, y, z), Arc::clone(&chunk_arc));
        }
         */

        // If we didn't find the chunk, we need to create it
        let chunk_arc = match chunk_arc_init {
            Some(chunk) => chunk,
            None => {
                let new_chunk = Arc::new(RwLock::new(ChunkState::new(x, y, z)));
                chunks_read.insert((x, y, z), Arc::clone(&new_chunk));
                new_chunk
            }
        };

        // By this point we have a chunk, but we don't know what state it is in.
        //let mut chunk_state = chunk_arc.lock().unwrap();
        //chunk_state.ensure_decorated();
        //drop(chunk_state);
        chunk_arc
    }

    // A faster version of ensure_chunk that does multiple chunks at once
    pub fn ensure_chunks(
        world: &Arc<World>,
        x_start: i32,
        x_end: i32,
        y_start: i32,
        y_end: i32,
        z_start: i32,
        z_end: i32,
    ) -> Vec<Arc<RwLock<ChunkState>>> {
        let chunks_read = world.chunks.read().unwrap();
        let mut chunk_arcs_init = Vec::new();
        for x in x_start..=x_end {
            for y in y_start..=y_end {
                for z in z_start..=z_end {
                    if let Some(chunk) = chunks_read.get(&(x, y, z)) {
                        chunk_arcs_init.push(((x, y, z), Some(Arc::clone(chunk))));
                    } else {
                        chunk_arcs_init.push(((x, y, z), None));
                    }
                }
            }
        }
        drop(chunks_read); // Drop the lock before awaiting

        let mut chunk_arcs = Vec::new();
        let mut world_write = world.chunks.write().unwrap();

        for ((x, y, z), chunk_arc_init) in chunk_arcs_init {
            let chunk_arc = match chunk_arc_init {
                Some(chunk) => chunk,
                None => {
                    let new_chunk = Arc::new(RwLock::new(ChunkState::new(x, y, z)));
                    world_write.insert((x, y, z), Arc::clone(&new_chunk));
                    new_chunk
                }
            };
            // By this point we have a chunk, but we don't know what state it is in.
            //let mut chunk_state = chunk_arc.lock().unwrap();
            //chunk_state.ensure_decorated();
            //drop(chunk_state);
            chunk_arcs.push(chunk_arc);
        }
        chunk_arcs
    }

    pub fn get_chunk(
        world: &Arc<World>,
        x: i32,
        y: i32,
        z: i32,
    ) -> Arc<RwLock<ChunkState>> {
        let chunk_arc = Self::ensure_chunk(world, x, y, z);

        ChunkState::ensure_formed(&mut chunk_arc.write().unwrap());

        chunk_arc
    }

    /*
    pub fn set_block(world: &Arc<World>, x: i32, y: i32, z: i32, block_id: u8) {
        let chunk_x = x.div_euclid(CHUNK_SIZE_X);
        let chunk_y = y.div_euclid(CHUNK_SIZE_X);
        let chunk_z = z.div_euclid(CHUNK_SIZE_X);
        let chunk = Self::get_chunk(world, chunk_x, chunk_y, chunk_z);
        chunk.set_block(
            x.rem_euclid(CHUNK_SIZE_X) as usize,
            y.rem_euclid(CHUNK_SIZE_X) as usize,
            z.rem_euclid(CHUNK_SIZE_X) as usize,
            block_id,
        );
    }
    */

    pub fn get_block(world: &Arc<World>, x: i32, y: i32, z: i32) -> u8 {
        let chunk_x = x.div_euclid(CHUNK_SIZE_X) as i32;
        let chunk_y = y.div_euclid(CHUNK_SIZE_X) as i32;
        let chunk_z = z.div_euclid(CHUNK_SIZE_X) as i32;
        let chunk = Self::get_chunk(world, chunk_x, chunk_y, chunk_z);
        chunk.read().unwrap().get_block(
            x.rem_euclid(CHUNK_SIZE_X) as usize,
            y.rem_euclid(CHUNK_SIZE_X) as usize,
            z.rem_euclid(CHUNK_SIZE_X) as usize,
        )
    }

    pub fn set_block(world: &Arc<World>, x: i32, y: i32, z: i32, block_id: u8) {
        let chunk_x = x.div_euclid(CHUNK_SIZE_X);
        let chunk_y = y.div_euclid(CHUNK_SIZE_X);
        let chunk_z = z.div_euclid(CHUNK_SIZE_X);
        let chunk = Self::get_chunk(world, chunk_x, chunk_y, chunk_z);
        let mut chunk_state = chunk.write().unwrap();
        chunk_state.set_block(
            x.rem_euclid(CHUNK_SIZE_X) as usize,
            y.rem_euclid(CHUNK_SIZE_X) as usize,
            z.rem_euclid(CHUNK_SIZE_X) as usize,
            block_id,
        );
        for listener in &world.chunk_update_listeners {
            let _ = listener.send(ChunkUpdateMessage {
                world: Arc::clone(world),
                x: chunk_x,
                y: chunk_y,
                z: chunk_z,
            });
        }
    }

    pub fn register_chunk_update_listener(
        &mut self,
    ) -> UnboundedReceiver<ChunkUpdateMessage> {
        let (sender, receiver) = unbounded_channel();
        self.chunk_update_listeners.push(sender);
        receiver
    }
}

pub struct WorldView {
    pub data: Vec<u8>,
    pub origin: (i32, i32, i32),
    pub size: (i32, i32, i32),
}

impl WorldView {
    pub async fn from_range(
        world: &Arc<World>,
        start_x: i32,
        end_x: i32,
        start_y: i32,
        end_y: i32,
        start_z: i32,
        end_z: i32,
    ) -> Self {
        // Calculate the size of the view
        let size_x = end_x - start_x + 1;
        let size_y = end_y - start_y + 1;
        let size_z = end_z - start_z + 1;

        // Calculate which chunks we need to cover this range
        let chunk_start_x = start_x.div_euclid(CHUNK_SIZE_X);
        let chunk_end_x = end_x.div_euclid(CHUNK_SIZE_X);
        let chunk_start_y = start_y.div_euclid(CHUNK_SIZE_X);
        let chunk_end_y = end_y.div_euclid(CHUNK_SIZE_X);
        let chunk_start_z = start_z.div_euclid(CHUNK_SIZE_X);
        let chunk_end_z = end_z.div_euclid(CHUNK_SIZE_X);

        //println!(
        //    "Range: {}..={}, chunks: {}..={}",
        //    start_x, end_x, chunk_start_x, chunk_end_x
        //);

        // Pre-allocate the data array
        let total_blocks = (size_x * size_y * size_z) as usize;
        let mut data = vec![0u8; total_blocks];

        // Get all required chunks using get_chunk to ensure proper decoration
        let mut chunk_arcs = Vec::new();
        for chunk_x in chunk_start_x..=chunk_end_x {
            for chunk_y in chunk_start_y..=chunk_end_y {
                for chunk_z in chunk_start_z..=chunk_end_z {
                    let chunk_arc = World::get_chunk(world, chunk_x, chunk_y, chunk_z);
                    chunk_arcs.push(chunk_arc);
                }
            }
        }

        // Create a map for fast chunk lookup
        let mut chunk_map = std::collections::HashMap::new();
        let mut chunk_index = 0;

        for chunk_x in chunk_start_x..=chunk_end_x {
            for chunk_y in chunk_start_y..=chunk_end_y {
                for chunk_z in chunk_start_z..=chunk_end_z {
                    chunk_map.insert((chunk_x, chunk_y, chunk_z), chunk_index);
                    chunk_index += 1;
                }
            }
        }

        // Lock all chunks and extract data (they're already formed and decorated)
        let mut chunk_guards = Vec::new();
        for chunk_arc in &chunk_arcs {
            let guard = chunk_arc.read().unwrap();
            chunk_guards.push(guard);
        }

        // Copy block data from chunks to our view
        for x in start_x..=end_x {
            for y in start_y..=end_y {
                for z in start_z..=end_z {
                    let chunk_x = x.div_euclid(CHUNK_SIZE_X);
                    let chunk_y = y.div_euclid(CHUNK_SIZE_X);
                    let chunk_z = z.div_euclid(CHUNK_SIZE_X);

                    let chunk_local_x = x.rem_euclid(CHUNK_SIZE_X) as usize;
                    let chunk_local_y = y.rem_euclid(CHUNK_SIZE_X) as usize;
                    let chunk_local_z = z.rem_euclid(CHUNK_SIZE_X) as usize;

                    // Find the chunk in our map
                    if let Some(&chunk_idx) =
                        chunk_map.get(&(chunk_x, chunk_y, chunk_z))
                    {
                        let chunk_guard = &chunk_guards[chunk_idx];
                        let block_id = chunk_guard.get_block(
                            chunk_local_x,
                            chunk_local_y,
                            chunk_local_z,
                        );

                        // Calculate index in our view data
                        let view_x = x - start_x;
                        let view_y = y - start_y;
                        let view_z = z - start_z;
                        let view_index =
                            (view_x + view_y * size_x + view_z * size_x * size_y)
                                as usize;

                        data[view_index] = block_id;
                    }
                }
            }
        }

        // Drop all chunk guards to release locks
        drop(chunk_guards);

        WorldView {
            data,
            origin: (start_x, start_y, start_z),
            size: (size_x, size_y, size_z),
        }
    }

    /// Get a block at the given world coordinates
    /// Returns 0 (air) if the coordinates are outside the view bounds
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> u8 {
        // Check if coordinates are within bounds
        let (origin_x, origin_y, origin_z) = self.origin;
        let (size_x, size_y, size_z) = self.size;

        if x < origin_x
            || x >= origin_x + size_x
            || y < origin_y
            || y >= origin_y + size_y
            || z < origin_z
            || z >= origin_z + size_z
        {
            return 0; // Outside bounds, return air
        }

        // Calculate local coordinates within the view
        let local_x = x - origin_x;
        let local_y = y - origin_y;
        let local_z = z - origin_z;

        // Calculate index in the data array
        let index = (local_x + local_y * size_x + local_z * size_x * size_y) as usize;

        // Return the block data
        self.data[index]
    }

    /// Check if the given world coordinates are within the view bounds
    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        let (origin_x, origin_y, origin_z) = self.origin;
        let (size_x, size_y, size_z) = self.size;

        x >= origin_x
            && x < origin_x + size_x
            && y >= origin_y
            && y < origin_y + size_y
            && z >= origin_z
            && z < origin_z + size_z
    }

    /// Get the bounds of this view
    pub fn bounds(&self) -> ((i32, i32, i32), (i32, i32, i32)) {
        let (origin_x, origin_y, origin_z) = self.origin;
        let (size_x, size_y, size_z) = self.size;
        (
            (origin_x, origin_y, origin_z),
            (
                origin_x + size_x - 1,
                origin_y + size_y - 1,
                origin_z + size_z - 1,
            ),
        )
    }

    /// Iterate over all blocks in the view
    pub fn iter_blocks(&self) -> impl Iterator<Item = (i32, i32, i32, u8)> + '_ {
        let (origin_x, origin_y, origin_z) = self.origin;
        let (size_x, size_y, size_z) = self.size;

        (0..size_x).flat_map(move |x| {
            (0..size_y).flat_map(move |y| {
                (0..size_z).map(move |z| {
                    let world_x = origin_x + x;
                    let world_y = origin_y + y;
                    let world_z = origin_z + z;
                    let block_id = self.get_block(world_x, world_y, world_z);
                    (world_x, world_y, world_z, block_id)
                })
            })
        })
    }
}

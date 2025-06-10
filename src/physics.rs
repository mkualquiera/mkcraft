use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use tokio::{spawn, sync::mpsc::UnboundedReceiver};

use crate::{
    tile::{self, TileRegistry},
    utils::QueuedItem,
    world::{CHUNK_SIZE, CHUNK_SIZE_X, ChunkUpdateMessage, World, WorldView},
};

pub struct PhysicsObject {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub collision_box: [[f32; 3]; 2],
}

struct VoxelCollisionChunk {
    pub is_solid: [bool; CHUNK_SIZE as usize],
}

pub struct RaycastHit {
    pub hit_point: [f32; 3],  // Exact hit coordinates (floats)
    pub voxel: [i32; 3],      // Voxel that was hit (ints)
    pub last_voxel: [i32; 3], // Last voxel before hit (ints)
    pub uv: [f32; 2],         // UV coordinates on the hit face (0.0-1.0)
    pub distance: f32,        // Distance from origin to hit
    pub face: usize,          // Which face was hit: 0=X, 1=Y, 2=Z
}

impl VoxelCollisionChunk {
    pub async fn from_world(
        world: Arc<World>,
        tile_registry: Arc<TileRegistry>,
        (chunk_x, chunk_y, chunk_z): (i32, i32, i32),
    ) -> Self {
        let mut data = [false; CHUNK_SIZE as usize];

        let start_x = chunk_x * CHUNK_SIZE_X;
        let start_y = chunk_y * CHUNK_SIZE_X;
        let start_z = chunk_z * CHUNK_SIZE_X;
        let end_x = start_x + CHUNK_SIZE_X;
        let end_y = start_y + CHUNK_SIZE_X;
        let end_z = start_z + CHUNK_SIZE_X;

        let view = WorldView::from_range(
            &world, start_x, end_x, start_y, end_y, start_z, end_z,
        )
        .await;

        for x in 0..CHUNK_SIZE_X {
            for y in 0..CHUNK_SIZE_X {
                for z in 0..CHUNK_SIZE_X {
                    let block_id =
                        view.get_block(start_x + x, start_y + y, start_z + z);
                    if block_id == 0 {
                        continue; // Skip air blocks
                    }
                    let tile =
                        tile_registry.get_handler(block_id).expect("Tile not found");
                    if tile.is_solid() {
                        let index =
                            (x + y * CHUNK_SIZE_X + z * CHUNK_SIZE_X * CHUNK_SIZE_X)
                                as usize;
                        data[index] = true;
                    }
                }
            }
        }

        VoxelCollisionChunk { is_solid: data }
    }
}

pub struct PhysicsEnvironment {
    collision_chunks:
        Arc<Mutex<HashMap<(i32, i32, i32), QueuedItem<VoxelCollisionChunk>>>>,
    tile_registry: Arc<TileRegistry>,
}

impl PhysicsEnvironment {
    async fn handle_chunk_updates(
        env: Arc<Self>,
        mut chunk_updates: UnboundedReceiver<ChunkUpdateMessage>,
    ) {
        loop {
            if let Some(chunk_update) = chunk_updates.recv().await {
                println!(
                    "[Physics] Chunk update received at position ({}, {}, {})",
                    chunk_update.x, chunk_update.y, chunk_update.z
                );
                // get current time to measure performance
                let start_time = std::time::Instant::now();
                let mut has_chunk = false;
                {
                    let chunks_handle = env.collision_chunks.lock().unwrap();
                    if chunks_handle.contains_key(&(
                        chunk_update.x,
                        chunk_update.y,
                        chunk_update.z,
                    )) {
                        has_chunk = true;
                    }
                }
                if !has_chunk {
                    continue;
                }
                let chunk = VoxelCollisionChunk::from_world(
                    chunk_update.world.clone(),
                    env.tile_registry.clone(),
                    (chunk_update.x, chunk_update.y, chunk_update.z),
                )
                .await;
                {
                    let mut chunks_handle = env.collision_chunks.lock().unwrap();
                    if chunks_handle.contains_key(&(
                        chunk_update.x,
                        chunk_update.y,
                        chunk_update.z,
                    )) {
                        chunks_handle.insert(
                            (chunk_update.x, chunk_update.y, chunk_update.z),
                            QueuedItem::Ready(chunk),
                        );
                    }
                }
                println!(
                    "[Physics] Chunk update processed for position ({}, {}, {}) in {} ms",
                    chunk_update.x,
                    chunk_update.y,
                    chunk_update.z,
                    start_time.elapsed().as_millis()
                );
            }
        }
    }

    pub fn new(
        chunk_updates: UnboundedReceiver<ChunkUpdateMessage>,
        tile_registry: Arc<TileRegistry>,
    ) -> Arc<Self> {
        let env = Arc::new(PhysicsEnvironment {
            collision_chunks: Arc::new(Mutex::new(HashMap::new())),
            tile_registry,
        });
        spawn(PhysicsEnvironment::handle_chunk_updates(
            env.clone(),
            chunk_updates,
        ));
        env
    }

    pub fn discard_chunk(&mut self, chunk_pos: (i32, i32, i32)) {
        self.collision_chunks.lock().unwrap().remove(&chunk_pos);
    }

    pub async fn solid_at(&self, x: i32, y: i32, z: i32) -> bool {
        let chunk_x = x.div_euclid(CHUNK_SIZE_X);
        let chunk_y = y.div_euclid(CHUNK_SIZE_X);
        let chunk_z = z.div_euclid(CHUNK_SIZE_X);

        let mut chunks_handle = self.collision_chunks.lock().unwrap();

        if let Some(chunk_ref) = chunks_handle.get_mut(&(chunk_x, chunk_y, chunk_z)) {
            if let Some(chunk) = chunk_ref.get().await {
                let local_x = x.rem_euclid(CHUNK_SIZE_X);
                let local_y = y.rem_euclid(CHUNK_SIZE_X);
                let local_z = z.rem_euclid(CHUNK_SIZE_X);
                return chunk.is_solid[(local_x
                    + local_y * CHUNK_SIZE_X
                    + local_z * CHUNK_SIZE_X * CHUNK_SIZE_X)
                    as usize];
            } else {
                return true;
            }
        }
        true // Default to solid if chunk not found
    }

    pub async fn is_colliding(
        &self,
        position: [f32; 3],
        collision_box: [[f32; 3]; 2],
    ) -> bool {
        let min = collision_box[0];
        let max = collision_box[1];
        let min = [
            (position[0] + min[0]),
            (position[1] + min[1]),
            (position[2] + min[2]),
        ];
        let max = [
            (position[0] + max[0]),
            (position[1] + max[1]),
            (position[2] + max[2]),
        ];

        // Use a small epsilon to handle floating point edge cases
        let epsilon = 1e-6;

        // Expand the range slightly to catch edge cases
        let min_bound = [
            (min[0] - epsilon).floor() as i32,
            (min[1] - epsilon).floor() as i32,
            (min[2] - epsilon).floor() as i32,
        ];
        let max_bound = [
            (max[0] + epsilon).floor() as i32,
            (max[1] + epsilon).floor() as i32,
            (max[2] + epsilon).floor() as i32,
        ];

        for x in min_bound[0]..=max_bound[0] {
            for y in min_bound[1]..=max_bound[1] {
                for z in min_bound[2]..=max_bound[2] {
                    if self.solid_at(x, y, z).await {
                        // Double-check that we actually overlap with this block
                        let block_min = [x as f32, y as f32, z as f32];
                        let block_max =
                            [x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0];

                        if min[0] < block_max[0]
                            && max[0] > block_min[0]
                            && min[1] < block_max[1]
                            && max[1] > block_min[1]
                            && min[2] < block_max[2]
                            && max[2] > block_min[2]
                        {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    pub async fn ensure_for_object(
        &self,
        world: Arc<World>,
        tile_registry: Arc<TileRegistry>,
        object: &PhysicsObject,
    ) {
        let chunk_x = (object.position[0].div_euclid(CHUNK_SIZE_X as f32)) as i32;
        let chunk_y = (object.position[1].div_euclid(CHUNK_SIZE_X as f32)) as i32;
        let chunk_z = (object.position[2].div_euclid(CHUNK_SIZE_X as f32)) as i32;

        let mut chunks_handle = self.collision_chunks.lock().unwrap();

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let chunk_coords = (chunk_x + dx, chunk_y + dy, chunk_z + dz);
                    if !chunks_handle.contains_key(&chunk_coords) {
                        chunks_handle.insert(
                            chunk_coords,
                            QueuedItem::enqueue(VoxelCollisionChunk::from_world(
                                world.clone(),
                                tile_registry.clone(),
                                chunk_coords,
                            )),
                        );
                    }
                }
            }
        }
    }

    pub async fn raycast(
        &self,
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
    ) -> Option<RaycastHit> {
        // Normalize direction vector
        let dir_length = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
            .sqrt();
        if dir_length == 0.0 {
            return None;
        }
        let dir = [
            direction[0] / dir_length,
            direction[1] / dir_length,
            direction[2] / dir_length,
        ];

        // Current voxel coordinates
        let mut voxel = [
            origin[0].floor() as i32,
            origin[1].floor() as i32,
            origin[2].floor() as i32,
        ];

        let mut last_voxel = voxel;

        // Direction to step in each axis
        let step = [
            if dir[0] >= 0.0 { 1 } else { -1 },
            if dir[1] >= 0.0 { 1 } else { -1 },
            if dir[2] >= 0.0 { 1 } else { -1 },
        ];

        // Calculate delta distances
        let delta = [
            (1.0 / dir[0]).abs(),
            (1.0 / dir[1]).abs(),
            (1.0 / dir[2]).abs(),
        ];

        // Calculate initial distances to next grid lines
        let mut max_dist = [0.0f32; 3];
        for i in 0..3 {
            if dir[i] >= 0.0 {
                max_dist[i] = (voxel[i] as f32 + 1.0 - origin[i]) * delta[i];
            } else {
                max_dist[i] = (origin[i] - voxel[i] as f32) * delta[i];
            }
        }

        let mut distance = 0.0;
        let mut hit_face = 0; // 0=x, 1=y, 2=z

        // DDA traversal
        while distance < max_distance {
            // Check if current voxel is solid
            if self.solid_at(voxel[0], voxel[1], voxel[2]).await {
                // Calculate exact hit point
                let hit_point = [
                    origin[0] + dir[0] * distance,
                    origin[1] + dir[1] * distance,
                    origin[2] + dir[2] * distance,
                ];

                // Calculate UV coordinates based on hit face
                let uv = match hit_face {
                    0 => [
                        // X face
                        (hit_point[2] - voxel[2] as f32).fract(),
                        (hit_point[1] - voxel[1] as f32).fract(),
                    ],
                    1 => [
                        // Y face
                        (hit_point[0] - voxel[0] as f32).fract(),
                        (hit_point[2] - voxel[2] as f32).fract(),
                    ],
                    2 => [
                        // Z face
                        (hit_point[0] - voxel[0] as f32).fract(),
                        (hit_point[1] - voxel[1] as f32).fract(),
                    ],
                    _ => [0.0, 0.0],
                };

                // Ensure UV coordinates are positive
                let uv = [
                    if uv[0] < 0.0 { uv[0] + 1.0 } else { uv[0] },
                    if uv[1] < 0.0 { uv[1] + 1.0 } else { uv[1] },
                ];

                return Some(RaycastHit {
                    hit_point,
                    voxel,
                    last_voxel,
                    uv,
                    distance,
                    face: hit_face,
                });
            }

            last_voxel = voxel;

            // Step to next voxel boundary
            if max_dist[0] < max_dist[1] && max_dist[0] < max_dist[2] {
                distance = max_dist[0];
                max_dist[0] += delta[0];
                voxel[0] += step[0];
                hit_face = 0;
            } else if max_dist[1] < max_dist[2] {
                distance = max_dist[1];
                max_dist[1] += delta[1];
                voxel[1] += step[1];
                hit_face = 1;
            } else {
                distance = max_dist[2];
                max_dist[2] += delta[2];
                voxel[2] += step[2];
                hit_face = 2;
            }
        }

        None
    }
}

impl PhysicsObject {
    pub fn new(
        position: [f32; 3],
        velocity: [f32; 3],
        collision_box: [[f32; 3]; 2],
    ) -> Self {
        Self {
            position,
            velocity,
            collision_box,
        }
    }

    fn resolve_axis_collision(
        current_pos: f32,
        velocity: f32,
        collision_box: [[f32; 3]; 2],
        axis: usize,
    ) -> f32 {
        if velocity < 0.0 {
            let box_edge = current_pos + collision_box[0][axis];
            let wall_coord = box_edge.floor() as i32;
            let ideal_pos = wall_coord as f32 - collision_box[0][axis];
            let penetration = current_pos - ideal_pos;

            if penetration > 1e-2 {
                // Deep penetration - push out to safe distance
                let corrected_pos = ideal_pos + 1e-2;
                let movement = corrected_pos - current_pos;
                //println!(
                //    "Deep penetration on axis {}: pushing out by {}",
                //    axis, movement
                //);
                movement
            } else {
                // Shallow penetration - just stop, don't push
                //println!("Shallow contact on axis {}: stopping only", axis);
                0.0
            }
        } else if velocity > 0.0 {
            let box_edge = current_pos + collision_box[1][axis];
            let wall_coord = box_edge.ceil() as i32;
            let ideal_pos = wall_coord as f32 - collision_box[1][axis];
            let penetration = ideal_pos - current_pos;

            if penetration > 1e-2 {
                // Deep penetration - push out to safe distance
                let corrected_pos = ideal_pos - 1e-2;
                let movement = corrected_pos - current_pos;
                //println!(
                //    "Deep penetration on axis {}: pushing out by {}",
                //    axis, movement
                //);
                movement
            } else {
                // Shallow penetration - just stop, don't push
                //println!("Shallow contact on axis {}: stopping only", axis);
                0.0
            }
        } else {
            0.0
        }
    }

    pub async fn update(&mut self, environment: &PhysicsEnvironment, delta_time: f32) {
        if environment
            .is_colliding(self.position, self.collision_box)
            .await
        {
            //println!(
            //    "WARNING: Already colliding at start of update! Position: {:?}",
            //    self.position
            //);
            // Try to push out of collision
            for axis in [1, 0, 2] {
                let original_pos = self.position[axis];
                // Try small adjustments in both directions
                for direction in [-1.0, 1.0] {
                    for distance in [0.01, 0.1, 0.5] {
                        self.position[axis] = original_pos + direction * distance;
                        if !environment
                            .is_colliding(self.position, self.collision_box)
                            .await
                        {
                            //println!(
                            //    "Pushed out of collision on axis {} by {}",
                            //    axis,
                            //    direction * distance
                            //);
                            return; // Exit early, don't do normal movement
                        }
                    }
                }
                self.position[axis] = original_pos; // Restore if no solution found
            }
        }
        let intended_movement = [
            self.velocity[0] * delta_time,
            self.velocity[1] * delta_time,
            self.velocity[2] * delta_time,
        ];

        let mut final_movement = intended_movement;

        // Check each axis independently
        for axis in [1, 0, 2] {
            let mut test_position = self.position;
            test_position[axis] += final_movement[axis];

            if environment
                .is_colliding(test_position, self.collision_box)
                .await
            {
                final_movement[axis] = PhysicsObject::resolve_axis_collision(
                    self.position[axis],
                    self.velocity[axis],
                    self.collision_box,
                    axis,
                );
                self.velocity[axis] = 0.0;
            }
        }

        // Apply the resolved movement all at once
        for axis in 0..3 {
            self.position[axis] += final_movement[axis];
        }
    }
}

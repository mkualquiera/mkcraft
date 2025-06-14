use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use gl33::GlFns;
use rand::Rng;
use tokio::{spawn, sync::mpsc::UnboundedReceiver};

use crate::{
    mesh::{MeshEnvelope, MeshParams},
    tile::{RenderLayer, TileFace, TileRegistry},
    utils::QueuedItem,
    world::{CHUNK_SIZE_X, ChunkUpdateMessage, World, WorldView},
};

const NEIGHBORHOOD_SCAN: [([(i32, i32, i32); 9], TileFace); 6] = [
    // Top face (y = 1) - for z in -1..=1, for x in -1..=1
    (
        [
            (-1, 1, -1),
            (0, 1, -1),
            (1, 1, -1), // z = -1
            (-1, 1, 0),
            (0, 1, 0),
            (1, 1, 0), // z = 0
            (-1, 1, 1),
            (0, 1, 1),
            (1, 1, 1), // z = 1
        ],
        TileFace::Top,
    ),
    // Bottom face (y = -1) - for z in (-1..=1).rev(), for x in -1..=1
    (
        [
            (-1, -1, 1),
            (0, -1, 1),
            (1, -1, 1), // z = 1
            (-1, -1, 0),
            (0, -1, 0),
            (1, -1, 0), // z = 0
            (-1, -1, -1),
            (0, -1, -1),
            (1, -1, -1), // z = -1
        ],
        TileFace::Bottom,
    ),
    // North face (z = -1) - for y in (-1..=1).rev(), for x in (-1..=1).rev()
    (
        [
            (1, 1, -1),
            (0, 1, -1),
            (-1, 1, -1), // y = 1
            (1, 0, -1),
            (0, 0, -1),
            (-1, 0, -1), // y = 0
            (1, -1, -1),
            (0, -1, -1),
            (-1, -1, -1), // y = -1
        ],
        TileFace::North,
    ),
    // West face (x = -1) - for y in (-1..=1).rev(), for z in -1..=1
    (
        [
            (-1, 1, -1),
            (-1, 1, 0),
            (-1, 1, 1), // y = 1
            (-1, 0, -1),
            (-1, 0, 0),
            (-1, 0, 1), // y = 0
            (-1, -1, -1),
            (-1, -1, 0),
            (-1, -1, 1), // y = -1
        ],
        TileFace::West,
    ),
    // South face (z = 1) - for y in (-1..=1).rev(), for x in -1..=1
    (
        [
            (-1, 1, 1),
            (0, 1, 1),
            (1, 1, 1), // y = 1
            (-1, 0, 1),
            (0, 0, 1),
            (1, 0, 1), // y = 0
            (-1, -1, 1),
            (0, -1, 1),
            (1, -1, 1), // y = -1
        ],
        TileFace::South,
    ),
    // East face (x = 1) - for y in (-1..=1).rev(), for z in (-1..=1).rev()
    (
        [
            (1, 1, 1),
            (1, 1, 0),
            (1, 1, -1), // y = 1
            (1, 0, 1),
            (1, 0, 0),
            (1, 0, -1), // y = 0
            (1, -1, 1),
            (1, -1, 0),
            (1, -1, -1), // y = -1
        ],
        TileFace::East,
    ),
];

struct TessellatedChunk {
    mesh: MeshEnvelope,
}

impl TessellatedChunk {
    pub async fn from_world(
        world: Arc<World>,
        tile_registry: Arc<TileRegistry>,
        (chunk_x, chunk_y, chunk_z): (i32, i32, i32),
        lod: u8,
    ) -> TessellatedChunk {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();
        let mut uvs = Vec::new();
        let mut materials = Vec::new();
        let mut lights = Vec::new();

        let chunk_basis_x = chunk_x * CHUNK_SIZE_X as i32;
        let chunk_basis_y = chunk_y * CHUNK_SIZE_X as i32;
        let chunk_basis_z = chunk_z * CHUNK_SIZE_X as i32;

        let worldview;
        worldview = WorldView::from_range(
            &world,
            chunk_basis_x - lod as i32,
            chunk_basis_x + CHUNK_SIZE_X + lod as i32,
            chunk_basis_y - lod as i32,
            chunk_basis_y + CHUNK_SIZE_X + lod as i32,
            chunk_basis_z - lod as i32,
            chunk_basis_z + CHUNK_SIZE_X + lod as i32,
        )
        .await;

        for x in (0..(CHUNK_SIZE_X as i32)).step_by(lod as usize) {
            for y in (0..(CHUNK_SIZE_X as i32)).step_by(lod as usize) {
                for z in (0..(CHUNK_SIZE_X as i32)).step_by(lod as usize) {
                    let block_x = chunk_basis_x + x;
                    let block_y = chunk_basis_y + y;
                    let block_z = chunk_basis_z + z;

                    //let block_id = Self::get_block(&world, block_x, block_y, block_z).await;
                    let block_id = worldview.get_block(block_x, block_y, block_z);

                    if block_id == 0 {
                        continue; // Skip air blocks
                    }

                    let tile_handler = tile_registry
                        .get_handler(block_id)
                        .expect("Tile handler not found");

                    for (neighborhood, face) in NEIGHBORHOOD_SCAN.iter() {
                        // see if neighbor 4 is air
                        let neighbor_x = block_x + neighborhood[4].0 * (lod as i32);
                        let neighbor_y = block_y + neighborhood[4].1 * (lod as i32);
                        let neighbor_z = block_z + neighborhood[4].2 * (lod as i32);
                        //let neighbor_block_id =
                        //    Self::get_block(&world, neighbor_x, neighbor_y, neighbor_z).await;
                        let neighbor_block_id =
                            worldview.get_block(neighbor_x, neighbor_y, neighbor_z);
                        if neighbor_block_id != 0 {
                            let direct_neighbor_handler =
                                tile_registry.get_handler(neighbor_block_id).expect(
                                    "Unable to find tile handler for neighbor block",
                                );
                            if direct_neighbor_handler
                                .occludes_geometry(RenderLayer::Opaque, block_id)
                            {
                                continue;
                            }
                        }

                        let mut neighbor_ids = [0; 9];
                        for (i, &(dx, dy, dz)) in neighborhood.iter().enumerate() {
                            let neighbor_x = block_x + dx * (lod as i32);
                            let neighbor_y = block_y + dy * (lod as i32);
                            let neighbor_z = block_z + dz * (lod as i32);

                            // Get the block ID of the neighboring block
                            //neighbor_ids[i] =
                            //    Self::get_block(&world, neighbor_x, neighbor_y, neighbor_z).await;
                            neighbor_ids[i] =
                                worldview.get_block(neighbor_x, neighbor_y, neighbor_z);
                        }

                        tile_handler.tesselate_face(
                            &tile_registry,
                            RenderLayer::Opaque,
                            block_id,
                            block_x as f32,
                            block_y as f32,
                            block_z as f32,
                            *face,
                            neighbor_ids,
                            block_id,
                            &mut vertices,
                            &mut indices,
                            &mut colors,
                            &mut uvs,
                            &mut materials,
                            &mut lights,
                            lod,
                        );
                    }
                }
            }
        }

        return Self {
            mesh: MeshEnvelope::new(MeshParams {
                vertices,
                indices: Some(indices),
                uvs: Some(uvs),
                material_ids: Some(materials),
                colors: Some(colors),
                light: Some(lights),
            }),
        };
    }
}

pub struct Tessellator {
    tessellated_chunks:
        Arc<Mutex<HashMap<(i32, i32, i32), HashMap<u8, QueuedItem<TessellatedChunk>>>>>,
    render_distance: i32,
    tile_registry: Arc<TileRegistry>,
}

impl Tessellator {
    pub async fn handle_chunk_updates(
        tessellator: Arc<Tessellator>,
        mut chunk_updates: UnboundedReceiver<ChunkUpdateMessage>,
    ) {
        loop {
            // just print for now
            if let Some(chunk_update) = chunk_updates.recv().await {
                println!(
                    "[Tessellator] Chunk update received at position ({}, {}, {})",
                    chunk_update.x, chunk_update.y, chunk_update.z
                );
                // get current time to measure performance
                let start_time = std::time::Instant::now();
                let mut lods_needed = Vec::new();
                {
                    let mut chunks_handle =
                        tessellator.tessellated_chunks.lock().unwrap();
                    let chunk_pos = (chunk_update.x, chunk_update.y, chunk_update.z);
                    if let Some(chunk_lods) = chunks_handle.get_mut(&chunk_pos) {
                        // If we have the chunk, we need to check which lods we need to update
                        for (&lod, _) in chunk_lods.iter() {
                            lods_needed.push(lod);
                        }
                    }
                }
                let mut lod_meshes = Vec::new();
                for lod in lods_needed {
                    for ox in -1..=1 {
                        for oy in -1..=1 {
                            for oz in -1..=1 {
                                let mesh_envelope = TessellatedChunk::from_world(
                                    Arc::clone(&chunk_update.world),
                                    Arc::clone(&tessellator.tile_registry),
                                    (
                                        chunk_update.x + ox,
                                        chunk_update.y + oy,
                                        chunk_update.z + oz,
                                    ),
                                    lod,
                                )
                                .await;
                                lod_meshes.push((
                                    (
                                        chunk_update.x + ox,
                                        chunk_update.y + oy,
                                        chunk_update.z + oz,
                                    ),
                                    lod,
                                    mesh_envelope,
                                ));
                            }
                        }
                    }
                }
                {
                    let mut chunks_handle =
                        tessellator.tessellated_chunks.lock().unwrap();
                    for (pos, lod, mesh_envelope) in lod_meshes {
                        if let Some(chunk_lods) = chunks_handle.get_mut(&pos) {
                            chunk_lods.insert(lod, QueuedItem::Ready(mesh_envelope));
                        }
                    }
                }
                println!(
                    "[Tessellator] Chunk update processed for position ({}, {}, {}) in {} ms",
                    chunk_update.x,
                    chunk_update.y,
                    chunk_update.z,
                    start_time.elapsed().as_millis()
                );
            }
        }
    }
    pub fn new(
        render_distance: i32,
        chunk_updates: UnboundedReceiver<ChunkUpdateMessage>,
        tile_registry: Arc<TileRegistry>,
    ) -> Arc<Self> {
        let tessellator = Arc::new(Tessellator {
            tessellated_chunks: Arc::new(Mutex::new(HashMap::new())),
            render_distance,
            tile_registry,
        });
        spawn(Self::handle_chunk_updates(
            tessellator.clone(),
            chunk_updates,
        ));
        tessellator
    }
    pub fn discard_chunk(&mut self, chunk_pos: (i32, i32, i32)) {
        self.tessellated_chunks.lock().unwrap().remove(&chunk_pos);
    }
    pub async fn render_chunks(
        &self,
        world: Arc<World>,
        tile_registry: Arc<TileRegistry>,
        (camera_pos_x, camera_pos_y, camera_pos_z): (f32, f32, f32),
        gl: &GlFns,
    ) -> usize {
        let mut unmet_meshes = 0;
        let mut queued_meshes = 0;
        let camera_chunk_pos = (
            (camera_pos_x as i32).div_euclid(CHUNK_SIZE_X),
            (camera_pos_y as i32).div_euclid(CHUNK_SIZE_X),
            (camera_pos_z as i32).div_euclid(CHUNK_SIZE_X),
        );

        let mut chunks_handle = self.tessellated_chunks.lock().unwrap();

        for x in -self.render_distance..self.render_distance {
            for z in -self.render_distance..self.render_distance {
                for y in -self.render_distance..self.render_distance {
                    let chunk_pos = (
                        camera_chunk_pos.0 + x,
                        camera_chunk_pos.1 + y,
                        camera_chunk_pos.2 + z,
                    );
                    let distance_to_camera = ((chunk_pos.0 - camera_chunk_pos.0).pow(2)
                        + (chunk_pos.1 - camera_chunk_pos.1).pow(2)
                        + (chunk_pos.2 - camera_chunk_pos.2).pow(2))
                        as f32;
                    let desired_lod: u8 = if distance_to_camera < (6 * 6) as f32 {
                        1
                    } else if distance_to_camera < (12 * 12) as f32 {
                        2
                    } else if distance_to_camera < (18 * 18) as f32 {
                        4
                    } else if distance_to_camera < (24 * 24) as f32 {
                        8
                    } else {
                        16
                    };
                    if !chunks_handle.contains_key(&chunk_pos) {
                        //let chunk_mesh = world.tesselate(&gl, &_tile_registry, chunk_pos, 2);
                        //tesselated_chunks.insert(chunk_pos, chunk_mesh);
                        chunks_handle.insert(chunk_pos, HashMap::new());
                    }
                    let mut rng = rand::rng();

                    // See if we have the chunk that we want
                    let found_lod =
                        if !chunks_handle[&chunk_pos].contains_key(&desired_lod) {
                            //if queued_meshes < 6 {
                            if rng.random_bool(0.1) {
                                // If not, spawn a thread to generate it
                                let handle =
                                    QueuedItem::enqueue(TessellatedChunk::from_world(
                                        Arc::clone(&world),
                                        Arc::clone(&tile_registry),
                                        chunk_pos,
                                        desired_lod,
                                    ));
                                chunks_handle
                                    .get_mut(&chunk_pos)
                                    .unwrap()
                                    .insert(desired_lod, handle);
                                queued_meshes += 1;
                            }
                            false
                        } else {
                            // If we have the chunk, check if it's ready
                            let queued_mesh = chunks_handle
                                .get_mut(&chunk_pos)
                                .unwrap()
                                .get_mut(&desired_lod)
                                .unwrap();

                            if let Some(mesh_envelope) = queued_mesh.get().await {
                                // If it's ready, render it
                                mesh_envelope.mesh.get_mesh(&gl).render(&gl);
                                true
                            } else {
                                false
                            }
                        };

                    if !found_lod {
                        // If we didn't find the chunk, we are happy to use any other lod
                        // starting from 1 then 2 then 4, etc.
                        for lod in [1, 2, 4, 8, 16] {
                            if let Some(queued_mesh) = chunks_handle.get_mut(&chunk_pos)
                            {
                                if let Some(queued_mesh) = queued_mesh.get_mut(&lod) {
                                    if let Some(mesh_envelope) = queued_mesh.get().await
                                    {
                                        mesh_envelope.mesh.get_mesh(&gl).render(&gl);
                                        break;
                                    } else {
                                        // If we are still generating, we can skip this lod
                                        unmet_meshes += 1;
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        unmet_meshes
    }
}

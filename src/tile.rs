use crate::utils::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFace {
    Top,
    Bottom,
    North,
    West,
    South,
    East,
}

pub enum RenderLayer {
    Opaque,
}

pub trait Tile: Sync + Send {
    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        // Default occlusion logic, can be overridden
        match render_layer {
            RenderLayer::Opaque => false, // Opaque tiles occlude geometry
        }
    }

    fn is_dual_sided(&self) -> bool {
        false
    }

    fn get_color_for_face(&self, _face: TileFace, _metadata: u8) -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0] // Default color, can be overridden
    }

    fn get_material_for_face(&self, _face: TileFace, _metadata: u8) -> [i32; 2] {
        [0, 0] // Default material, can be overridden
    }

    fn is_solid(&self) -> bool {
        false
    }

    fn occlude_vertex(&self, occluded_neighbors: i32) -> [f32; 4] {
        // Default occlusion logic, can be overridden
        //if occluded_neighbors > 1 {
        //    [0.4, 0.4, 0.4, 1.0] // Darker color for occluded vertices
        //} else {
        //    [1.0, 1.0, 1.0, 1.0] // Normal color for non-occluded vertices
        //}
        match occluded_neighbors {
            0 => [0.975, 0.975, 0.975, 1.0], // Fully lit
            1 => [0.8, 0.8, 0.8, 1.0],       // Slightly occluded
            2 => [0.7, 0.7, 0.7, 1.0],       // More occluded
            _ => [0.65, 0.65, 0.65, 1.0],    // Heavily occluded
        }
    }

    fn occlusion_filter(&self, input_color: &[f32; 4]) -> [f32; 4] {
        // Default occlusion filter, can be overridden
        // This could apply some kind of lighting or shading effect
        *input_color
    }

    fn tesselate_face(
        &self,
        tile_registry: &TileRegistry,
        render_layer: RenderLayer,
        block_id: u8,
        x: f32,
        y: f32,
        z: f32,
        face: TileFace,
        neigbor_ids: [u8; 9],
        metadata: u8,
        vertices: &mut Vec<[f32; 3]>,
        indices: &mut Vec<u32>,
        colors: &mut Vec<[f32; 4]>,
        uvs: &mut Vec<[f32; 2]>,
        materials: &mut Vec<[i32; 2]>,
        lights: &mut Vec<[f32; 4]>,
        lod: u8,
    ) {
        let lod = lod as f32;
        let mut neighbor_handler = None;
        if neigbor_ids[4] != 0 {
            neighbor_handler = Some(
                tile_registry
                    .get_handler(neigbor_ids[4])
                    .expect("Unable to find tile handler"),
            );
            if neighbor_handler
                .unwrap()
                .occludes_geometry(render_layer, block_id)
            {
                return; // No need to tesselate if the neighbor occludes geometry
            }
        }
        let vertex_count = vertices.len() as u32;
        match face {
            TileFace::Top => {
                vertices.push([
                    BACK_TOP_LEFT_X * lod + x as f32,
                    BACK_TOP_LEFT_Y * lod + y as f32,
                    BACK_TOP_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_TOP_RIGHT_X * lod + x as f32,
                    BACK_TOP_RIGHT_Y * lod + y as f32,
                    BACK_TOP_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_TOP_RIGHT_X * lod + x as f32,
                    FRONT_TOP_RIGHT_Y * lod + y as f32,
                    FRONT_TOP_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_TOP_LEFT_X * lod + x as f32,
                    FRONT_TOP_LEFT_Y * lod + y as f32,
                    FRONT_TOP_LEFT_Z * lod + z as f32,
                ]);
            }
            TileFace::Bottom => {
                vertices.push([
                    FRONT_BOTTOM_LEFT_X * lod + x as f32,
                    FRONT_BOTTOM_LEFT_Y * lod + y as f32,
                    FRONT_BOTTOM_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_BOTTOM_RIGHT_X * lod + x as f32,
                    FRONT_BOTTOM_RIGHT_Y * lod + y as f32,
                    FRONT_BOTTOM_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_BOTTOM_RIGHT_X * lod + x as f32,
                    BACK_BOTTOM_RIGHT_Y * lod + y as f32,
                    BACK_BOTTOM_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_BOTTOM_LEFT_X * lod + x as f32,
                    BACK_BOTTOM_LEFT_Y * lod + y as f32,
                    BACK_BOTTOM_LEFT_Z * lod + z as f32,
                ]);
            }
            TileFace::North => {
                vertices.push([
                    FRONT_BOTTOM_RIGHT_X * lod + x as f32,
                    FRONT_BOTTOM_RIGHT_Y * lod + y as f32,
                    FRONT_BOTTOM_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_BOTTOM_LEFT_X * lod + x as f32,
                    FRONT_BOTTOM_LEFT_Y * lod + y as f32,
                    FRONT_BOTTOM_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_TOP_LEFT_X * lod + x as f32,
                    FRONT_TOP_LEFT_Y * lod + y as f32,
                    FRONT_TOP_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_TOP_RIGHT_X * lod + x as f32,
                    FRONT_TOP_RIGHT_Y * lod + y as f32,
                    FRONT_TOP_RIGHT_Z * lod + z as f32,
                ]);
            }
            TileFace::West => {
                vertices.push([
                    FRONT_BOTTOM_LEFT_X * lod + x as f32,
                    FRONT_BOTTOM_LEFT_Y * lod + y as f32,
                    FRONT_BOTTOM_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_BOTTOM_LEFT_X * lod + x as f32,
                    BACK_BOTTOM_LEFT_Y * lod + y as f32,
                    BACK_BOTTOM_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_TOP_LEFT_X * lod + x as f32,
                    BACK_TOP_LEFT_Y * lod + y as f32,
                    BACK_TOP_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_TOP_LEFT_X * lod + x as f32,
                    FRONT_TOP_LEFT_Y * lod + y as f32,
                    FRONT_TOP_LEFT_Z * lod + z as f32,
                ]);
            }
            TileFace::South => {
                vertices.push([
                    BACK_BOTTOM_LEFT_X * lod + x as f32,
                    BACK_BOTTOM_LEFT_Y * lod + y as f32,
                    BACK_BOTTOM_LEFT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_BOTTOM_RIGHT_X * lod + x as f32,
                    BACK_BOTTOM_RIGHT_Y * lod + y as f32,
                    BACK_BOTTOM_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_TOP_RIGHT_X * lod + x as f32,
                    BACK_TOP_RIGHT_Y * lod + y as f32,
                    BACK_TOP_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_TOP_LEFT_X * lod + x as f32,
                    BACK_TOP_LEFT_Y * lod + y as f32,
                    BACK_TOP_LEFT_Z * lod + z as f32,
                ]);
            }
            TileFace::East => {
                vertices.push([
                    BACK_BOTTOM_RIGHT_X * lod + x as f32,
                    BACK_BOTTOM_RIGHT_Y * lod + y as f32,
                    BACK_BOTTOM_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_BOTTOM_RIGHT_X * lod + x as f32,
                    FRONT_BOTTOM_RIGHT_Y * lod + y as f32,
                    FRONT_BOTTOM_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    FRONT_TOP_RIGHT_X * lod + x as f32,
                    FRONT_TOP_RIGHT_Y * lod + y as f32,
                    FRONT_TOP_RIGHT_Z * lod + z as f32,
                ]);
                vertices.push([
                    BACK_TOP_RIGHT_X * lod + x as f32,
                    BACK_TOP_RIGHT_Y * lod + y as f32,
                    BACK_TOP_RIGHT_Z * lod + z as f32,
                ]);
            }
        }
        // compute ambient occlusion
        let ao_bottom_left_coords: i32 = [
            if neigbor_ids[3] == 0 { 0 } else { 1 },
            if neigbor_ids[4] == 0 { 0 } else { 1 },
            if neigbor_ids[6] == 0 { 0 } else { 1 },
            if neigbor_ids[7] == 0 { 0 } else { 1 },
        ]
        .iter()
        .sum();
        let ao_bottom_right_coords: i32 = [
            if neigbor_ids[4] == 0 { 0 } else { 1 },
            if neigbor_ids[5] == 0 { 0 } else { 1 },
            if neigbor_ids[7] == 0 { 0 } else { 1 },
            if neigbor_ids[8] == 0 { 0 } else { 1 },
        ]
        .iter()
        .sum();
        let ao_top_right_coords: i32 = [
            if neigbor_ids[1] == 0 { 0 } else { 1 },
            if neigbor_ids[2] == 0 { 0 } else { 1 },
            if neigbor_ids[4] == 0 { 0 } else { 1 },
            if neigbor_ids[5] == 0 { 0 } else { 1 },
        ]
        .iter()
        .sum();
        let ao_top_left_coords: i32 = [
            if neigbor_ids[0] == 0 { 0 } else { 1 },
            if neigbor_ids[1] == 0 { 0 } else { 1 },
            if neigbor_ids[3] == 0 { 0 } else { 1 },
            if neigbor_ids[4] == 0 { 0 } else { 1 },
        ]
        .iter()
        .sum();

        indices.push(vertex_count);
        indices.push(vertex_count + 1);
        indices.push(vertex_count + 2);
        indices.push(vertex_count + 2);
        indices.push(vertex_count + 3);
        indices.push(vertex_count);
        if self.is_dual_sided() {
            indices.push(vertex_count + 3);
            indices.push(vertex_count + 2);
            indices.push(vertex_count + 1);
            indices.push(vertex_count + 1);
            indices.push(vertex_count);
            indices.push(vertex_count + 3);
        }
        colors.push(self.get_color_for_face(face, metadata));
        colors.push(self.get_color_for_face(face, metadata));
        colors.push(self.get_color_for_face(face, metadata));
        colors.push(self.get_color_for_face(face, metadata));
        uvs.push([0.0 * (lod as f32), 1.0 * (lod as f32)]);
        uvs.push([1.0 * (lod as f32), 1.0 * (lod as f32)]);
        uvs.push([1.0 * (lod as f32), 0.0 * (lod as f32)]);
        uvs.push([0.0 * (lod as f32), 0.0 * (lod as f32)]);
        materials.push(self.get_material_for_face(face, metadata));
        materials.push(self.get_material_for_face(face, metadata));
        materials.push(self.get_material_for_face(face, metadata));
        materials.push(self.get_material_for_face(face, metadata));
        let run_filter = |x: &[f32; 4]| {
            if let Some(tile) = neighbor_handler {
                tile.occlusion_filter(x)
            } else {
                *x
            }
        };
        lights.push(run_filter(&self.occlude_vertex(ao_bottom_left_coords)));
        lights.push(run_filter(&self.occlude_vertex(ao_bottom_right_coords)));
        lights.push(run_filter(&self.occlude_vertex(ao_top_right_coords)));
        lights.push(run_filter(&self.occlude_vertex(ao_top_left_coords)));
    }
}

pub struct TileRegistry {
    handlers: [Option<Box<dyn Tile>>; 256], // Fixed size array
}

pub struct StoneTile;
impl Tile for StoneTile {
    fn get_material_for_face(&self, _face: TileFace, _metadata: u8) -> [i32; 2] {
        [1, 0]
    }

    fn is_solid(&self) -> bool {
        true
    }

    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        match render_layer {
            RenderLayer::Opaque => true,
        }
    }
}
pub struct DirtTile;
impl Tile for DirtTile {
    fn get_material_for_face(&self, _face: TileFace, _metadata: u8) -> [i32; 2] {
        [2, 0] // Example material ID for dirt
    }

    fn is_solid(&self) -> bool {
        true
    }

    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        match render_layer {
            RenderLayer::Opaque => true,
        }
    }
}
pub struct GrassTile;
impl Tile for GrassTile {
    fn get_color_for_face(&self, _face: TileFace, _metadata: u8) -> [f32; 4] {
        [0.36, 0.62, 0.1, 1.0] // Green color for grass
    }
    fn get_material_for_face(&self, face: TileFace, _metadata: u8) -> [i32; 2] {
        match face {
            TileFace::Top => [0, 0],
            TileFace::Bottom => [2, 0],
            _ => [3, 0],
        }
    }
    fn is_solid(&self) -> bool {
        true
    }
    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        match render_layer {
            RenderLayer::Opaque => true,
        }
    }
}
pub struct WaterTile;
impl Tile for WaterTile {
    fn get_material_for_face(&self, face: TileFace, _metadata: u8) -> [i32; 2] {
        match face {
            _ => [15, 13],
        }
    }
    fn is_solid(&self) -> bool {
        false // Water is not solid
    }
    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        match render_layer {
            RenderLayer::Opaque => target == 4,
        }
    }

    fn occlusion_filter(&self, input_color: &[f32; 4]) -> [f32; 4] {
        // Apply a blue tint for water
        [
            input_color[0],
            input_color[1] * 1.1,
            input_color[1] * 1.5, // Increase blue
            input_color[3],       // Keep alpha
        ]
    }

    fn is_dual_sided(&self) -> bool {
        true // Water is dual-sided
    }
}

pub struct LogTile;
impl Tile for LogTile {
    fn get_material_for_face(&self, _face: TileFace, _metadata: u8) -> [i32; 2] {
        match _face {
            TileFace::Top | TileFace::Bottom => [5, 1],
            _ => [4, 1], // Example material ID for log sides
        }
    }
    fn is_solid(&self) -> bool {
        true
    }
    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        match render_layer {
            RenderLayer::Opaque => true,
        }
    }
}

pub struct LeavesTile;
impl Tile for LeavesTile {
    fn get_color_for_face(&self, _face: TileFace, _metadata: u8) -> [f32; 4] {
        // green-yellowish
        [141.0 / 255.0, 191.0 / 255.0, 43.0 / 255.0, 1.0]
    }
    fn get_material_for_face(&self, _face: TileFace, _metadata: u8) -> [i32; 2] {
        [4, 3]
        //[0, 5]
    }
    fn is_solid(&self) -> bool {
        true
    }
    fn occludes_geometry(&self, render_layer: RenderLayer, target: u8) -> bool {
        match render_layer {
            RenderLayer::Opaque => {
                // Only occludes if it's myself (target == 6)
                target == 6
            }
        }
    }
    fn is_dual_sided(&self) -> bool {
        true
    }
}

impl TileRegistry {
    pub fn new() -> Self {
        const INIT: Option<Box<dyn Tile>> = None;
        let mut registry = TileRegistry {
            handlers: [INIT; 256],
        };

        // Register default tiles
        registry.handlers[1] = Some(Box::new(StoneTile));
        registry.handlers[2] = Some(Box::new(DirtTile));
        registry.handlers[3] = Some(Box::new(GrassTile));
        registry.handlers[4] = Some(Box::new(WaterTile));
        registry.handlers[5] = Some(Box::new(LogTile));
        registry.handlers[6] = Some(Box::new(LeavesTile));

        registry
    }

    pub fn get_handler(&self, id: u8) -> Option<&dyn Tile> {
        self.handlers[id as usize].as_deref()
    }
}

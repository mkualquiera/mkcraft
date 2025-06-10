use gl33::*;
use std::mem::size_of;

pub type Vertex = [f32; 3];
pub type UV = [f32; 2];
pub type Color = [f32; 4];
pub type MaterialId = [i32; 2];

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vao: u32,
    pub vbo: u32,
    pub ebo: Option<u32>,
    pub index_count: i32,
    pub vertex_count: i32,
}

impl Mesh {
    pub fn new(
        gl: &GlFns,
        vertices: &[Vertex],
        indices: Option<&[u32]>,
        uvs: Option<&[UV]>,
        material_ids: Option<&[MaterialId]>,
        colors: Option<&[Color]>,
        light: Option<&[Color]>,
    ) -> Self {
        if vertices.is_empty() {
            return Mesh {
                vao: 0,
                vbo: 0,
                ebo: None,
                index_count: 0,
                vertex_count: 0,
            };
        }
        unsafe {
            let mut vao = 0;
            gl.GenVertexArrays(1, &mut vao);
            gl.BindVertexArray(vao);

            let mut vbo = 0;
            gl.GenBuffers(1, &mut vbo);
            gl.BindBuffer(GL_ARRAY_BUFFER, vbo);
            gl.BufferData(
                GL_ARRAY_BUFFER,
                (vertices.len() * size_of::<Vertex>()) as isize,
                vertices.as_ptr().cast(),
                GL_STATIC_DRAW,
            );

            // Position attribute (location 0)
            gl.VertexAttribPointer(
                0,
                3,
                GL_FLOAT,
                GL_FALSE.0 as u8,
                size_of::<Vertex>() as i32,
                0 as *const _,
            );
            gl.EnableVertexAttribArray(0);

            let mut ebo = None;
            let (index_count, vertex_count) = if let Some(indices) = indices {
                let mut ebo_id = 0;
                gl.GenBuffers(1, &mut ebo_id);
                gl.BindBuffer(GL_ELEMENT_ARRAY_BUFFER, ebo_id);
                gl.BufferData(
                    GL_ELEMENT_ARRAY_BUFFER,
                    (indices.len() * size_of::<u32>()) as isize,
                    indices.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                ebo = Some(ebo_id);
                (indices.len() as i32, vertices.len() as i32)
            } else {
                (0, vertices.len() as i32)
            };

            // UVs (location 1) - always set up the attribute even if no data
            if let Some(uvs) = uvs {
                let mut uv_vbo = 0;
                gl.GenBuffers(1, &mut uv_vbo);
                gl.BindBuffer(GL_ARRAY_BUFFER, uv_vbo);
                gl.BufferData(
                    GL_ARRAY_BUFFER,
                    (uvs.len() * size_of::<UV>()) as isize,
                    uvs.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                gl.VertexAttribPointer(
                    1,
                    2,
                    GL_FLOAT,
                    GL_FALSE.0 as u8,
                    size_of::<UV>() as i32,
                    0 as *const _,
                );
                gl.EnableVertexAttribArray(1);
            }

            // Material IDs (location 2)
            if let Some(material_ids) = material_ids {
                let mut material_vbo = 0;
                gl.GenBuffers(1, &mut material_vbo);
                gl.BindBuffer(GL_ARRAY_BUFFER, material_vbo);
                gl.BufferData(
                    GL_ARRAY_BUFFER,
                    (material_ids.len() * size_of::<MaterialId>()) as isize,
                    material_ids.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                gl.VertexAttribIPointer(
                    2,
                    2,
                    GL_INT,
                    size_of::<MaterialId>() as i32,
                    0 as *const _,
                );
                gl.EnableVertexAttribArray(2);
            }

            // Colors (location 3)
            if let Some(colors) = colors {
                let mut color_vbo = 0;
                gl.GenBuffers(1, &mut color_vbo);
                gl.BindBuffer(GL_ARRAY_BUFFER, color_vbo);
                gl.BufferData(
                    GL_ARRAY_BUFFER,
                    (colors.len() * size_of::<Color>()) as isize,
                    colors.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                gl.VertexAttribPointer(
                    3,
                    4,
                    GL_FLOAT,
                    GL_FALSE.0 as u8,
                    size_of::<Color>() as i32,
                    0 as *const _,
                );
                gl.EnableVertexAttribArray(3);
            }

            // Light (location 4), same as colors
            if let Some(light) = light {
                let mut light_vbo = 0;
                gl.GenBuffers(1, &mut light_vbo);
                gl.BindBuffer(GL_ARRAY_BUFFER, light_vbo);
                gl.BufferData(
                    GL_ARRAY_BUFFER,
                    (light.len() * size_of::<Color>()) as isize,
                    light.as_ptr().cast(),
                    GL_STATIC_DRAW,
                );
                gl.VertexAttribPointer(
                    4,
                    4,
                    GL_FLOAT,
                    GL_FALSE.0 as u8,
                    size_of::<Color>() as i32,
                    0 as *const _,
                );
                gl.EnableVertexAttribArray(4);
            }

            gl.BindVertexArray(0);

            Mesh {
                vao,
                vbo,
                ebo,
                index_count,
                vertex_count,
            }
        }
    }

    pub fn render(&self, gl: &GlFns) {
        if self.vertex_count == 0 {
            return; // No mesh to render
        }
        unsafe {
            gl.BindVertexArray(self.vao);
            if let Some(_) = self.ebo {
                gl.DrawElements(
                    GL_TRIANGLES,
                    self.index_count,
                    GL_UNSIGNED_INT,
                    0 as *const _,
                );
            } else {
                gl.DrawArrays(GL_TRIANGLES, 0, self.vertex_count);
            }
        }
    }

    pub fn update_colors(&self, gl: &GlFns, colors: &[Color]) {
        unsafe {
            gl.BindVertexArray(self.vao);
            let mut color_vbo = 0;
            gl.GenBuffers(1, &mut color_vbo);
            gl.BindBuffer(GL_ARRAY_BUFFER, color_vbo);
            gl.BufferData(
                GL_ARRAY_BUFFER,
                (colors.len() * size_of::<Color>()) as isize,
                colors.as_ptr().cast(),
                GL_DYNAMIC_DRAW,
            );
            gl.VertexAttribPointer(
                3,
                4,
                GL_FLOAT,
                GL_FALSE.0 as u8,
                size_of::<Color>() as i32,
                0 as *const _,
            );
            gl.EnableVertexAttribArray(3);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        // Note: This requires a GL context to be current
        // In a real game, you'd want proper resource management
    }
}

pub struct MeshParams {
    pub vertices: Vec<Vertex>,
    pub indices: Option<Vec<u32>>,
    pub uvs: Option<Vec<UV>>,
    pub material_ids: Option<Vec<MaterialId>>,
    pub colors: Option<Vec<Color>>,
    pub light: Option<Vec<Color>>,
}

pub enum MeshEnvelope {
    Parameters(MeshParams),
    Mesh(Mesh),
}

impl MeshEnvelope {
    pub fn new(params: MeshParams) -> Self {
        Self::Parameters(params)
    }

    pub fn get_mesh(&mut self, gl: &GlFns) -> &Mesh {
        match self {
            MeshEnvelope::Parameters(params) => {
                let mesh = Mesh::new(
                    gl,
                    &params.vertices,
                    params.indices.as_deref(),
                    params.uvs.as_deref(),
                    params.material_ids.as_deref(),
                    params.colors.as_deref(),
                    params.light.as_deref(),
                );
                *self = MeshEnvelope::Mesh(mesh);
                if let MeshEnvelope::Mesh(m) = self {
                    m
                } else {
                    unreachable!()
                }
            }
            MeshEnvelope::Mesh(mesh) => mesh,
        }
    }
}

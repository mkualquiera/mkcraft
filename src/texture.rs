use std::collections::HashMap;

use gl33::*;
use imagine::{Bitmap, png::png_try_bitmap_rgba};
use pixel_formats::r8g8b8a8_Srgb;

use crate::shader::Shader;

pub struct Texture {
    pub id: u32,
    pub texture_type: GLenum,
}

impl Texture {
    pub fn new(gl: &GlFns) -> Self {
        unsafe {
            let mut id = 0;
            gl.GenTextures(1, &mut id);
            Texture {
                id,
                texture_type: GL_TEXTURE_2D,
            }
        }
    }

    pub fn from_data(
        gl: &GlFns,
        width: i32,
        height: i32,
        data: &[u8],
        format: GLenum,
    ) -> Self {
        let texture = Self::new(gl);
        texture.bind(gl);

        unsafe {
            gl.TexImage2D(
                GL_TEXTURE_2D,
                0,
                format.0 as i32,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                data.as_ptr().cast(),
            );
            // Remove gl.GenerateMipmap(GL_TEXTURE_2D); - don't need mipmaps for pixel art

            // Set texture parameters for pixel art
            gl.TexParameteri(
                GL_TEXTURE_2D,
                GL_TEXTURE_WRAP_S,
                GL_CLAMP_TO_EDGE.0 as i32,
            );
            gl.TexParameteri(
                GL_TEXTURE_2D,
                GL_TEXTURE_WRAP_T,
                GL_CLAMP_TO_EDGE.0 as i32,
            );
            gl.TexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST.0 as i32);
            gl.TexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST.0 as i32);
        }

        texture
    }

    pub fn create_solid_color(gl: &GlFns, r: u8, g: u8, b: u8, a: u8) -> Self {
        let data = [r, g, b, a];
        Self::from_data(gl, 1, 1, &data, GL_RGBA)
    }

    pub fn bind(&self, gl: &GlFns) {
        unsafe {
            gl.BindTexture(self.texture_type, self.id);
        }
    }

    pub fn bind_to_unit(&self, gl: &GlFns, unit: u32) {
        unsafe {
            gl.ActiveTexture(GLenum(GL_TEXTURE0.0 + unit));
            self.bind(gl);
        }
    }

    pub fn unbind(&self, gl: &GlFns) {
        unsafe {
            gl.BindTexture(self.texture_type, 0);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        // Note: This requires a GL context to be current
        // In a real game, you'd want proper resource management
    }
}

pub struct TextureManager {
    textures: HashMap<String, Texture>,
}

impl TextureManager {
    pub fn new(gl: &GlFns) -> Self {
        let mut manager = TextureManager {
            textures: HashMap::new(),
        };
        manager.load_png_texture(
            gl,
            "terrain",
            include_bytes!("assets/textures/terrain.png"),
        );
        manager.load_png_texture(
            gl,
            "font",
            include_bytes!("assets/textures/font.png"),
        );
        manager
    }

    pub fn load_texture(
        &mut self,
        gl: &GlFns,
        name: &str,
        width: i32,
        height: i32,
        data: &[u8],
        format: GLenum,
    ) -> usize {
        let texture = Texture::from_data(gl, width, height, data, format);
        self.textures.insert(name.to_string(), texture);
        self.textures.len() - 1
    }

    pub fn get_texture(&self, index: usize) -> Option<&Texture> {
        self.textures.values().nth(index)
    }

    pub fn get_texture_by_name(&self, name: &str) -> Option<&Texture> {
        self.textures.get(name)
    }

    pub fn bind_texture(&self, gl: &GlFns, index: usize, unit: u32) {
        if let Some(texture) = self.get_texture(index) {
            texture.bind_to_unit(gl, unit);
        }
    }

    pub fn bind_texture_by_name(&self, gl: &GlFns, name: &str, unit: u32) {
        if let Some(texture) = self.get_texture_by_name(name) {
            texture.bind_to_unit(gl, unit);
        }
    }

    pub fn load_png_texture(&mut self, gl: &GlFns, name: &str, bytes: &[u8]) -> usize {
        let bitmap: Bitmap<r8g8b8a8_Srgb> =
            png_try_bitmap_rgba(bytes, true).expect("Failed to decode PNG texture");

        let width = bitmap.width;
        let height = bitmap.height;
        let data = bitmap.pixels;

        let mut output_data = Vec::with_capacity(width as usize * height as usize * 4);

        for pixel in data {
            output_data.push(pixel.r);
            output_data.push(pixel.g);
            output_data.push(pixel.b);
            output_data.push(pixel.a);
        }

        self.load_texture(
            gl,
            name,
            width as i32,
            height as i32,
            &output_data,
            GL_SRGB8_ALPHA8,
        )
    }

    pub fn set_texture_uniform(
        &self,
        gl: &GlFns,
        texture_name: &str,
        shader_program: Shader,
        uniform_name: &str,
        texture_unit: u32,
    ) {
        if let Some(texture) = self.get_texture_by_name(&texture_name) {
            texture.bind_to_unit(gl, texture_unit);
            shader_program.set_int(gl, uniform_name, texture_unit as i32);
        } else {
            eprintln!("Texture '{}' not found", texture_name);
        }
    }
}

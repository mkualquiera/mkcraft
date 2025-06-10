use gl33::*;
use std::ffi::CString;
use ultraviolet::Mat4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shader {
    pub program_id: u32,
}

impl Shader {
    pub fn new(
        gl: &GlFns,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<Self, String> {
        unsafe {
            let vertex_shader =
                Self::compile_shader(gl, GL_VERTEX_SHADER, vertex_source)?;
            let fragment_shader =
                Self::compile_shader(gl, GL_FRAGMENT_SHADER, fragment_source)?;

            let program_id = gl.CreateProgram();
            if program_id == 0 {
                return Err("Failed to create shader program".to_string());
            }

            gl.AttachShader(program_id, vertex_shader);
            gl.AttachShader(program_id, fragment_shader);
            gl.LinkProgram(program_id);

            let mut success = 0;
            gl.GetProgramiv(program_id, GL_LINK_STATUS, &mut success);
            if success == 0 {
                let mut v: Vec<u8> = Vec::with_capacity(1024);
                let mut log_len = 0_i32;
                gl.GetProgramInfoLog(
                    program_id,
                    1024,
                    &mut log_len,
                    v.as_mut_ptr().cast(),
                );
                v.set_len(log_len.try_into().unwrap());
                return Err(format!(
                    "Shader Program Link Error: {}",
                    String::from_utf8_lossy(&v)
                ));
            }

            gl.DeleteShader(vertex_shader);
            gl.DeleteShader(fragment_shader);

            Ok(Shader { program_id })
        }
    }

    pub fn from_files(
        gl: &GlFns,
        _vertex_path: &str,
        _fragment_path: &str,
    ) -> Result<Self, String> {
        let vertex_source = include_str!("assets/shaders/vertex_test.glsl"); // This would be dynamic in a real implementation
        let fragment_source = include_str!("assets/shaders/fragment_test.glsl");
        Self::new(gl, vertex_source, fragment_source)
    }

    fn compile_shader(
        gl: &GlFns,
        shader_type: GLenum,
        source: &str,
    ) -> Result<u32, String> {
        unsafe {
            let shader = gl.CreateShader(shader_type);
            if shader == 0 {
                return Err("Failed to create shader".to_string());
            }

            gl.ShaderSource(
                shader,
                1,
                &(source.as_bytes().as_ptr().cast()),
                &(source.len().try_into().unwrap()),
            );
            gl.CompileShader(shader);

            let mut success = 0;
            gl.GetShaderiv(shader, GL_COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut v: Vec<u8> = Vec::with_capacity(1024);
                let mut log_len = 0_i32;
                gl.GetShaderInfoLog(shader, 1024, &mut log_len, v.as_mut_ptr().cast());
                v.set_len(log_len.try_into().unwrap());
                let shader_type_name = if shader_type == GL_VERTEX_SHADER {
                    "Vertex"
                } else {
                    "Fragment"
                };
                return Err(format!(
                    "{} Compile Error: {}",
                    shader_type_name,
                    String::from_utf8_lossy(&v)
                ));
            }

            Ok(shader)
        }
    }

    pub fn use_program(&self, gl: &GlFns) {
        gl.UseProgram(self.program_id);
    }

    pub fn get_uniform_location(&self, gl: &GlFns, name: &str) -> i32 {
        unsafe {
            let c_name = CString::new(name).unwrap();
            gl.GetUniformLocation(self.program_id, c_name.as_ptr().cast())
        }
    }

    pub fn set_mat4(&self, gl: &GlFns, name: &str, matrix: &Mat4) {
        unsafe {
            let location = self.get_uniform_location(gl, name);
            gl.UniformMatrix4fv(location, 1, GL_FALSE.0 as u8, matrix.as_ptr());
        }
    }

    pub fn set_vec3(&self, gl: &GlFns, name: &str, value: &[f32; 3]) {
        unsafe {
            let location = self.get_uniform_location(gl, name);
            gl.Uniform3fv(location, 1, value.as_ptr());
        }
    }

    pub fn set_float(&self, gl: &GlFns, name: &str, value: f32) {
        unsafe {
            let location = self.get_uniform_location(gl, name);
            gl.Uniform1f(location, value);
        }
    }

    pub fn set_int(&self, gl: &GlFns, name: &str, value: i32) {
        unsafe {
            let location = self.get_uniform_location(gl, name);
            gl.Uniform1i(location, value);
        }
    }

    pub fn unset_mat4(&self, gl: &GlFns, name: &str) {
        unsafe {
            let location = self.get_uniform_location(gl, name);
            gl.UniformMatrix4fv(location, 1, GL_FALSE.0 as u8, std::ptr::null());
        }
    }
}

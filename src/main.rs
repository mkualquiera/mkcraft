use std::{collections::HashMap, env, sync::Arc};

use beryllium::*;
use gl33::*;
use physics::{PhysicsEnvironment, PhysicsObject};
use text::{Text, TextOptions};
use tokio::task::JoinHandle;
use ultraviolet::{Mat4, projection};

mod camera;
mod mesh;
mod physics;
mod shader;
mod tessellator;
mod text;
mod texture;
mod tile;
mod toki;
mod utils;
mod world;

use camera::{Camera, CameraMovement};
use mesh::{Mesh, MeshEnvelope};
use shader::Shader;
use texture::TextureManager;
use utils::QueuedItem;
use world::{CHUNK_SIZE_X, World};

use crate::{physics::RaycastHit, tessellator::Tessellator, text::into_syllabic};
const RENDER_DISTANCE: i32 = 16; // Number of chunks to render in each direction

//enum QueuedMesh {
//    Generating(JoinHandle<MeshEnvelope>),
//    Ready(MeshEnvelope),
//}

//impl QueuedMesh {
//    async fn advance(&mut self) {
//        match self {
//            QueuedMesh::Generating(handle) => {
//                if handle.is_finished() {
//                    let mesh_envelope = handle.await.expect("Failed to join thread");
//                    *self = QueuedMesh::Ready(mesh_envelope);
//                }
//            }
//            QueuedMesh::Ready(_) => (),
//        }
//    }
//}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    //println!("{:?}", Text::from_spec("o kama seli\no kama pona").unwrap());

    let sdl = Sdl::init(init::InitFlags::EVERYTHING);
    sdl.set_gl_context_major_version(3).unwrap();
    sdl.set_gl_profile(video::GlProfile::Core).unwrap();

    let mut camera = Camera::new();
    let mut delta_time;
    let mut last_frame = std::time::Instant::now();

    let mut keys_pressed = std::collections::HashSet::new();

    let win_args = video::CreateWinArgs {
        title: &env::args().next().unwrap_or_else(|| "mkcraft".to_string()),
        width: 800,
        height: 600,
        allow_high_dpi: true,
        borderless: false,
        resizable: false,
    };

    let _win = sdl
        .create_gl_window(win_args)
        .expect("couldn't make a window and context");

    let gl = unsafe {
        GlFns::load_from(&|s| _win.get_proc_address(s)).expect("Unable to load gl")
    };

    // Initialize OpenGL settings
    unsafe {
        gl.ClearColor(148.0 / 255.0, 243.0 / 255.0, 255.0 / 255.0, 1.0);
        //gl.ClearColor(255.0 / 255.0, 126.0 / 255.0, 33.0 / 255.0, 1.0);
        //gl.ClearColor(0.51, 0.86, 0.9, 1.0);
        gl.Enable(GL_DEPTH_TEST);
        gl.Enable(GL_CULL_FACE);
        gl.Enable(GL_BLEND);
        gl.BlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);
    }

    // Create shader program
    let vertex_source = include_str!("assets/shaders/vertex_test.glsl");
    let fragment_source = include_str!("assets/shaders/fragment_test.glsl");
    let shader = Shader::new(&gl, vertex_source, fragment_source)
        .expect("Failed to create shader program");

    let text_vertex_source = include_str!("assets/shaders/vertex_test.glsl");
    let text_fragment_source = include_str!("assets/shaders/fragment_text.glsl");
    let text_shader = Shader::new(&gl, text_vertex_source, text_fragment_source)
        .expect("Failed to create text shader");

    // Create mesh
    //let quad_mesh = create_quad_mesh(&gl);

    let tile_registry = Arc::new(tile::TileRegistry::new());
    let mut _world = World::new();
    let tessellator = Tessellator::new(
        RENDER_DISTANCE,
        _world.register_chunk_update_listener(),
        tile_registry.clone(),
    );
    let mut physics_env = PhysicsEnvironment::new(
        _world.register_chunk_update_listener(),
        tile_registry.clone(),
    );
    let world = Arc::new(_world);

    //let test_chunk_mesh = world.tesselate(&gl, (0, 0, 0));
    //let mut test_chunks = Vec::new();
    //for x in -1..=1 {
    //    for z in -1..=1 {
    //        let chunk = world.tesselate(&gl, (x, 0, z));
    //        test_chunks.push(chunk);
    //    }
    //}

    let mut player_obj = PhysicsObject {
        position: [0.0, 25.0, 0.0],
        velocity: [0.0, 0.0, 0.0],
        collision_box: [[-0.3, -1.64, -0.3], [0.3, 1.8 - 1.62, 0.3]],
    };

    let mut time: f32 = 0.0;

    let mut test_text = TextOptions::new(15)
        .set_alignment(text::Alignment::Top)
        .render_spec("o pona kama tawa musi leko pona mi a")
        .expect("Failed to create text");
    let mut test_text2 = TextOptions::new(15)
        .set_alignment(text::Alignment::Bottom)
        .set_origin(text::MeshOrigin::BL)
        .render_spec("f:#ff0000ff ma li pali mute... o awen.")
        .expect("Failed to create text");

    // Create texture manager (for future use)
    let texture_manager = TextureManager::new(&gl);

    camera.movement_speed = 5.0; // Set camera movement speed

    sdl.set_relative_mouse_mode(true).unwrap();

    'main_loop: loop {
        let current_frame = std::time::Instant::now();
        delta_time = current_frame.duration_since(last_frame).as_secs_f32();
        last_frame = current_frame;

        shader.use_program(&gl);
        texture_manager.set_texture_uniform(
            &gl,
            "terrain",
            shader,
            "terrainTexture",
            0,
        );

        shader.set_float(&gl, "time", time);
        shader.set_vec3(
            &gl,
            "cameraPos",
            &[camera.position.x, camera.position.y, camera.position.z],
        );

        let mut breaking_block = false;
        let mut placing_block = false;

        // handle events this frame
        while let Some(event) = sdl.poll_events() {
            match event {
                (events::Event::Quit, _) => break 'main_loop,
                (
                    events::Event::Key {
                        pressed, keycode, ..
                    },
                    _,
                ) => match keycode {
                    events::SDLK_w => {
                        if pressed {
                            keys_pressed.insert('w');
                        } else {
                            keys_pressed.remove(&'w');
                        }
                    }
                    events::SDLK_s => {
                        if pressed {
                            keys_pressed.insert('s');
                        } else {
                            keys_pressed.remove(&'s');
                        }
                    }
                    events::SDLK_a => {
                        if pressed {
                            keys_pressed.insert('a');
                        } else {
                            keys_pressed.remove(&'a');
                        }
                    }
                    events::SDLK_d => {
                        if pressed {
                            keys_pressed.insert('d');
                        } else {
                            keys_pressed.remove(&'d');
                        }
                    }
                    events::SDLK_SPACE => {
                        if pressed {
                            player_obj.velocity[1] = 9.0; // Jump
                        }
                    }
                    _ => (),
                },
                (
                    events::Event::MouseMotion {
                        x_delta, y_delta, ..
                    },
                    _,
                ) => {
                    camera.process_mouse_movement(x_delta as f32, -(y_delta as f32));
                }
                (
                    events::Event::MouseButton {
                        win_id,
                        mouse_id,
                        button,
                        pressed,
                        clicks,
                        x,
                        y,
                    },
                    _,
                ) => {
                    //println!(
                    //    "Mouse Button Event: win_id: {}, mouse_id: {}, button: {:?}, pressed: {}, clicks: {}, x: {}, y: {}",
                    //    win_id, mouse_id, button, pressed, clicks, x, y
                    //);
                    if button == 1 {
                        if pressed {
                            // Handle left click (e.g., breaking a block)
                            breaking_block = true;
                        }
                    } else if button == 3 {
                        if pressed {
                            // Handle right click (e.g., placing a block)
                            placing_block = true;
                        }
                    }
                }
                _ => (),
            }
        }
        // now the events are clear

        let front = camera.front;
        let right = camera.right;

        const PLAYER_SPEED: f32 = 4.31; // Speed of the player

        let mut intended_velocity = [0.0, 0.0, 0.0];

        // Process continuous key input
        if keys_pressed.contains(&'w') {
            let front_player =
                ultraviolet::Vec3::new(front.x, 0.0, front.z).normalized();

            intended_velocity[0] += front_player.x * PLAYER_SPEED;
            intended_velocity[2] += front_player.z * PLAYER_SPEED;
        }
        if keys_pressed.contains(&'s') {
            let back_player =
                ultraviolet::Vec3::new(-front.x, 0.0, -front.z).normalized();

            intended_velocity[0] += back_player.x * PLAYER_SPEED;
            intended_velocity[2] += back_player.z * PLAYER_SPEED;
        }
        if keys_pressed.contains(&'a') {
            let left_player =
                ultraviolet::Vec3::new(-right.x, 0.0, -right.z).normalized();
            intended_velocity[0] += left_player.x * PLAYER_SPEED;
            intended_velocity[2] += left_player.z * PLAYER_SPEED;
        }
        if keys_pressed.contains(&'d') {
            let right_player =
                ultraviolet::Vec3::new(right.x, 0.0, right.z).normalized();

            intended_velocity[0] += right_player.x * PLAYER_SPEED;
            intended_velocity[2] += right_player.z * PLAYER_SPEED;
        }

        if !keys_pressed.contains(&'w')
            && !keys_pressed.contains(&'s')
            && !keys_pressed.contains(&'a')
            && !keys_pressed.contains(&'d')
        {
        } else {
            if !(intended_velocity[0] == 0.0 && intended_velocity[2] == 0.0) {
                let intended_normed = ultraviolet::Vec3::new(
                    intended_velocity[0],
                    0.0,
                    intended_velocity[2],
                )
                .normalized();
                player_obj.velocity[0] += intended_normed.x * PLAYER_SPEED;
                player_obj.velocity[2] += intended_normed.z * PLAYER_SPEED;
            }
        }
        // Apply friction
        player_obj.velocity[0] *= 0.5; // Friction on X
        player_obj.velocity[2] *= 0.5; // Friction on Z

        unsafe {
            gl.Clear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        }

        // Use shader and set uniforms
        shader.use_program(&gl);

        let model = Mat4::identity();
        let view = camera.get_view_matrix();
        let projection = projection::rh_yup::perspective_gl(
            90.0_f32.to_radians(),
            800.0 / 600.0,
            0.1,
            (CHUNK_SIZE_X * RENDER_DISTANCE) as f32,
        );
        let mvp = projection * view * model;

        shader.set_mat4(&gl, "mvp", &mvp);

        // Render the quad
        //test_chunk_mesh.render(&gl);
        //for chunk in &test_chunks {
        //    chunk.render(&gl);
        //}

        physics_env
            .ensure_for_object(world.clone(), tile_registry.clone(), &player_obj)
            .await;

        //println!(
        //    "Player Position: {:?}, Velocity: {:?}",
        //    player_obj.position, player_obj.velocity
        //);

        player_obj.velocity[1] -= 32.6 * delta_time; // Simple gravity
        player_obj.update(&physics_env, delta_time).await;

        camera.position = ultraviolet::Vec3::new(
            player_obj.position[0],
            player_obj.position[1],
            player_obj.position[2],
        );

        let unmet_meshes = tessellator
            .render_chunks(
                Arc::clone(&world),
                Arc::clone(&tile_registry),
                (camera.position.x, camera.position.y, camera.position.z),
                &gl,
            )
            .await;

        if let Some(result) = physics_env
            .raycast(camera.position.into(), camera.front.into(), 4.0)
            .await
        {
            //println!(
            //    "Raycast hit: {:?} at distance: {}",
            //    result.hit_point, result.distance
            //);
            let hit_as_float = [
                result.voxel[0] as f32,
                result.voxel[1] as f32,
                result.voxel[2] as f32,
            ];
            shader.set_vec3(&gl, "cursorPos", &hit_as_float);
            if breaking_block {
                World::set_block(
                    &world,
                    result.voxel[0],
                    result.voxel[1],
                    result.voxel[2],
                    0,
                );
            } else if placing_block {
                World::set_block(
                    &world,
                    result.last_voxel[0],
                    result.last_voxel[1],
                    result.last_voxel[2],
                    1,
                );
            }
        }

        text_shader.use_program(&gl);
        texture_manager.set_texture_uniform(
            &gl,
            "font",
            text_shader,
            "terrainTexture",
            0,
        );

        let gui_projection =
            projection::rh_yup::orthographic_gl(0.0, 800.0, 0.0, 600.0, -1.0, 1.0);

        let test_scale = Mat4::from_scale(16.0);

        let test_translation = Mat4::from_translation(ultraviolet::Vec3::new(
            800.0 - 64.0,
            600.0 - 64.0,
            0.0,
        ));

        let gui_mvp = gui_projection * test_translation * test_scale;

        text_shader.set_mat4(&gl, "mvp", &gui_mvp);
        //shader.set_mat4(&gl, "mvp", &gui_projection);

        test_text.get_mesh(&gl).render(&gl);

        let test_translation = Mat4::from_translation(ultraviolet::Vec3::new(
            20.0 + 64.0,
            20.0 + 64.0,
            0.0,
        ));

        let gui_mvp = gui_projection * test_translation * test_scale;

        text_shader.set_mat4(&gl, "mvp", &gui_mvp);

        if unmet_meshes > 0 {
            test_text2.get_mesh(&gl).render(&gl);
        }

        time += delta_time;

        _win.swap_window();
    }
}

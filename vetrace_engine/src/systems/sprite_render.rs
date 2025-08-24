#![cfg(not(feature = "wgpu"))]

use crate::components::components::{Sprite3D, Transform};
use crate::ecs::Behaviour;
use crate::engine::engine::Engine;
use crate::math::*;
use crate::rendering::resource::{compile_shader, link_program};
use gl::types::*;
use glam::{Mat4, Quat, Vec3};
use std::ffi::CString;
use std::mem::size_of;

#[repr(C)]
struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

pub struct SpriteRenderSystem {
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    initialized: bool,
}

impl SpriteRenderSystem {
    pub fn new() -> Self {
        Self {
            program: 0,
            vao: 0,
            vbo: 0,
            initialized: false,
        }
    }

    fn init(&mut self) {
        unsafe {
            let vert_src = include_str!("../../assets/shaders/opengl/sprite/sprite_vert.glsl",);
            let frag_src = include_str!("../../assets/shaders/opengl/sprite/sprite_frag.glsl",);
            let vs = compile_shader(vert_src, gl::VERTEX_SHADER);
            let fs = compile_shader(frag_src, gl::FRAGMENT_SHADER);
            self.program = link_program(vs, fs);
            gl::DeleteShader(vs);
            gl::DeleteShader(fs);

            gl::GenVertexArrays(1, &mut self.vao);
            gl::GenBuffers(1, &mut self.vbo);
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            let stride = (3 + 2) * size_of::<GLfloat>() as GLsizei;
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (3 * size_of::<GLfloat>()) as *const _,
            );
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
        self.initialized = true;
    }
}

impl Behaviour for SpriteRenderSystem {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        if !self.initialized {
            self.init();
        }
        let cam = engine.active_camera_info();
        let front = cam.orientation * Vec3::X;
        let up_v = cam.orientation * Vec3::Y;
        let right_v = cam.orientation * Vec3::Z;
        let (w, h) = engine.window.get_size();
        let view_proj: Mat4 = if engine.is_2d {
            let scale = h as f32 / (cam.fov * 10.0);
            let sx = 2.0 * scale / w as f32;
            let sy = 2.0 * scale / h as f32;
            Mat4::from_cols_array(&[
                sx,
                0.0,
                0.0,
                0.0,
                0.0,
                sy,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                0.0,
                -cam.position.x * sx,
                -cam.position.y * sy,
                0.0,
                1.0,
            ])
        } else {
            let aspect = w as f32 / h as f32;
            perspective(cam.fov, aspect, 0.1, 1000.0)
                * look_at(&cam.position, &(cam.position + front), &up_v)
        };
        let vp = view_proj.to_cols_array();
        unsafe {
            gl::UseProgram(self.program);
            let loc =
                gl::GetUniformLocation(self.program, CString::new("viewProj").unwrap().as_ptr());
            gl::UniformMatrix4fv(loc, 1, gl::FALSE, vp.as_ptr());
            let cam_loc =
                gl::GetUniformLocation(self.program, CString::new("camPos").unwrap().as_ptr());
            gl::Uniform3fv(cam_loc, 1, cam.position.to_array().as_ptr());
            let depth_loc =
                gl::GetUniformLocation(self.program, CString::new("rayDepth").unwrap().as_ptr());
            gl::Uniform1i(depth_loc, 1);
            let mode_loc =
                gl::GetUniformLocation(self.program, CString::new("is2D").unwrap().as_ptr());
            gl::Uniform1i(mode_loc, if engine.is_2d { 1 } else { 0 });
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, engine.renderer.depth_texture);
            gl::BindVertexArray(self.vao);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
        }

        let global_map: std::collections::HashMap<_, _> = engine
            .world
            .query::<crate::components::components::GlobalTransform>()
            .into_iter()
            .map(|(e, g)| (e, (g.position, g.orientation)))
            .collect();

        for (e, t, sprite) in engine.world.query2_mut::<Transform, Sprite3D>() {
            let (pos, ori) = global_map
                .get(&e)
                .copied()
                .unwrap_or((t.position, t.orientation));
            let pos = Vec3::from(pos);
            let mut right = Vec3::X;
            let mut up = Vec3::Y;
            if engine.is_2d {
                if !sprite.facing_camera {
                    let angle = 2.0 * ori[2].atan2(ori[3]);
                    let q = Quat::from_rotation_z(angle);
                    right = q * right;
                    up = q * up;
                }
            } else if sprite.facing_camera {
                right = right_v;
                up = up_v;
            } else {
                let q = Quat::from_xyzw(ori[0], ori[1], ori[2], ori[3]);
                right = q * right;
                up = q * up;
            }
            right = right.normalize() * sprite.size[0] * t.size[0] * 0.5;
            up = up.normalize() * sprite.size[1] * t.size[1] * 0.5;

            let p0 = pos - right - up;
            let p1 = pos + right - up;
            let p2 = pos - right + up;
            let p3 = pos + right + up;

            let verts = [
                // Triangles wound counter-clockwise so the sprite faces -Z by default
                Vertex {
                    pos: p0.into(),
                    uv: [0.0, 0.0],
                },
                Vertex {
                    pos: p2.into(),
                    uv: [0.0, 1.0],
                },
                Vertex {
                    pos: p1.into(),
                    uv: [1.0, 0.0],
                },
                Vertex {
                    pos: p2.into(),
                    uv: [0.0, 1.0],
                },
                Vertex {
                    pos: p3.into(),
                    uv: [1.0, 1.0],
                },
                Vertex {
                    pos: p1.into(),
                    uv: [1.0, 0.0],
                },
            ];

            unsafe {
                if sprite.double_sided {
                    gl::Disable(gl::CULL_FACE);
                } else {
                    gl::Enable(gl::CULL_FACE);
                }
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, sprite.texture.0);
                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (verts.len() * size_of::<Vertex>()) as isize,
                    verts.as_ptr() as *const _,
                    gl::DYNAMIC_DRAW,
                );
                gl::DrawArrays(gl::TRIANGLES, 0, 6);
            }
        }
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::UseProgram(0);
            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::DEPTH_TEST);
        }
    }
}

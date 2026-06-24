use crate::rendering::resource::{compile_shader, link_program};
use crate::rendering::ssbo::{create_ssbo, update_ssbo};
use crate::scene::bvh::GpuBvhNode;
use crate::scene::object::{GpuAtmosphere, GpuObject, GpuTriangle, GpuVolumetricCloud};
use crate::scene::tri_bvh::GpuTriBvhNode;
use gl::types::*;
use sdl2::video::Window;
use std::ffi::CString;
use std::mem::size_of;
use std::ptr;

fn load_gl(window: &Window) {
    gl::load_with(|s| window.subsystem().gl_get_proc_address(s) as *const _);
}

fn create_texture(
    width: i32,
    height: i32,
    internal: GLint,
    format: GLenum,
    ty: GLenum,
    filter: GLint,
) -> GLuint {
    unsafe {
        let mut tex = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            internal,
            width,
            height,
            0,
            format,
            ty,
            ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        tex
    }
}

fn resize_texture(
    tex: GLuint,
    width: i32,
    height: i32,
    internal: GLint,
    format: GLenum,
    ty: GLenum,
) {
    unsafe {
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            internal,
            width,
            height,
            0,
            format,
            ty,
            ptr::null(),
        );
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
}

pub struct RenderParams {
    pub camera_pos: [f32; 3],
    pub camera_front: [f32; 3],
    pub camera_up: [f32; 3],
    pub camera_right: [f32; 3],
    pub velocity: [f32; 3],
    pub fov: f32,
    pub num_objects: i32,
    pub current_time: f32,
    pub skycolor: [f32; 3],
    pub is_fisheye: i32,
    pub selected_index: i32,
    pub max_bounces: i32,
    pub light_samples: i32,
    pub dir_shadow_samples: i32,
    pub inv_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
    pub gi_quality: u32,
    pub gi_debug_mode: u32,
    pub gi_mode: u32,
    pub dir_light_dir: [f32; 3],
    pub dir_light_color: [f32; 3],
    pub dir_light_intensity: f32,
    pub sky_occlusion: f32,
    pub dof_aperture: f32,
    pub dof_focus_dist: f32,
    pub dof_enable: u32,
    pub atmos: Vec<GpuAtmosphere>,
    pub atmosphere: u32,
    /// 0 = precomputed LUT atmosphere, 1 = inline atmosphere integration for debug/quality A/B tests.
    pub atmosphere_mode: u32,
    /// x = sun angular radius in radians, y = sun disk intensity, z = sky luminance scale.
    pub atmosphere_sun_controls: [f32; 4],
    pub clouds: Vec<GpuVolumetricCloud>,
}

struct Uniforms {
    camera_pos: GLint,
    camera_front: GLint,
    camera_up: GLint,
    camera_right: GLint,
    camera_velocity: GLint,
    fov: GLint,
    is_fisheye: GLint,
    skycolor: GLint,
    taa_jitter: GLint,
    current_time: GLint,
    frame_number: GLint,
    num_objects: GLint,
}

struct DenoiseUniforms {
    taa_jitter: GLint,
    frame_number: GLint,
}

pub struct Renderer {
    screen_width: i32,
    screen_height: i32,
    render_width: i32,
    render_height: i32,
    render_scale: f32,
    frame_number: i32,
    is_2d: bool,
    use_fsr: bool,
    sharpness: f32,
    ray_program: GLuint,
    denoise_program: GLuint,
    quad_program: GLuint,
    texture: GLuint,
    ray_texture: GLuint,
    pub depth_texture: GLuint,
    pub normal_texture: GLuint,
    capture_texture: GLuint,
    blur_program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    pub object_ssbo: GLuint,
    pub triangle_ssbo: GLuint,
    pub bvh_ssbo: GLuint,
    pub tri_bvh_ssbo: GLuint,
    uniforms: Uniforms,
    denoise_uniforms: DenoiseUniforms,
    prev_triangles: Vec<GpuTriangle>,
    prev_bvh: Vec<GpuBvhNode>,
    prev_tri_bvh: Vec<GpuTriBvhNode>,
}

impl Renderer {
    pub fn new(window: &Window, screen_width: i32, screen_height: i32, is_2d: bool) -> Self {
        load_gl(window);

        // Set up OpenGL state
        unsafe {
            gl::Viewport(0, 0, screen_width, screen_height);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
        }
        let ray_src: &str = if is_2d {
            include_str!("../../assets/shaders/opengl/raytracing/compute_shader_2d.glsl")
        } else {
            include_str!("../../assets/shaders/opengl/raytracing/compute_shader.glsl")
        };
        let denoise_src: &str =
            include_str!("../../assets/shaders/opengl/raytracing/denoise_shader.glsl",);
        const vert_src: &str =
            include_str!("../../assets/shaders/opengl/raster/quad_vertex_shader.glsl");
        const frag_src: &str =
            include_str!("../../assets/shaders/opengl/raster/quad_fragment_shader.glsl");
        const blur_vert_src: &str = include_str!("../../assets/shaders/opengl/ui/blur.vert");
        const blur_frag_src: &str = include_str!("../../assets/shaders/opengl/ui/blur.frag");

        let ray_shader = compile_shader(&ray_src, gl::COMPUTE_SHADER);
        let denoise_shader = compile_shader(&denoise_src, gl::COMPUTE_SHADER);
        let quad_vert_shader = compile_shader(&vert_src, gl::VERTEX_SHADER);
        let quad_frag_shader = compile_shader(&frag_src, gl::FRAGMENT_SHADER);
        let blur_vert_shader = compile_shader(&blur_vert_src, gl::VERTEX_SHADER);
        let blur_frag_shader = compile_shader(&blur_frag_src, gl::FRAGMENT_SHADER);

        let ray_program = link_program(ray_shader, 0);
        let denoise_program = link_program(denoise_shader, 0);
        let quad_program = link_program(quad_vert_shader, quad_frag_shader);
        let blur_program = link_program(blur_vert_shader, blur_frag_shader);
        let warn_missing = is_2d;
        let get_uniform = |name: &str| -> GLint {
            let c_name = CString::new(name).unwrap();
            let loc = unsafe { gl::GetUniformLocation(ray_program, c_name.as_ptr()) };
            if loc == -1 && warn_missing {
                println!("Warning: Uniform '{}' not found", name);
            }
            loc
        };

        let uniforms = Uniforms {
            camera_pos: get_uniform("camera_pos"),
            camera_front: get_uniform("camera_front"),
            camera_up: get_uniform("camera_up"),
            camera_right: get_uniform("camera_right"),
            camera_velocity: get_uniform("camera_velocity"),
            fov: get_uniform("fov"),
            is_fisheye: get_uniform("is_fisheye"),
            skycolor: get_uniform("skycolor"),
            taa_jitter: get_uniform("taa_jitter"),
            current_time: get_uniform("currentTime"),
            frame_number: get_uniform("frameNumber"),
            num_objects: get_uniform("num_objects"),
        };

        let get_denoise_uniform = |name: &str| -> GLint {
            let c_name = CString::new(name).unwrap();
            let loc = unsafe { gl::GetUniformLocation(denoise_program, c_name.as_ptr()) };
            if loc == -1 && warn_missing {
                println!("Warning: Uniform '{}' not found", name);
            }
            loc
        };

        let denoise_uniforms = DenoiseUniforms {
            taa_jitter: get_denoise_uniform("taa_jitter"),
            frame_number: get_denoise_uniform("frameNumber"),
        };

        let render_width = screen_width;
        let render_height = screen_height;

        let texture = create_texture(
            render_width,
            render_height,
            gl::RGBA32F as GLint,
            gl::RGBA,
            gl::FLOAT,
            gl::NEAREST as GLint,
        );

        let ray_texture = create_texture(
            render_width,
            render_height,
            gl::RGBA32F as GLint,
            gl::RGBA,
            gl::FLOAT,
            gl::NEAREST as GLint,
        );

        let depth_texture = create_texture(
            render_width,
            render_height,
            gl::R32F as GLint,
            gl::RED,
            gl::FLOAT,
            gl::NEAREST as GLint,
        );

        let normal_texture = create_texture(
            render_width,
            render_height,
            gl::RGBA32F as GLint,
            gl::RGBA,
            gl::FLOAT,
            gl::NEAREST as GLint,
        );

        let capture_texture = create_texture(
            screen_width,
            screen_height,
            gl::RGBA as GLint,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            gl::LINEAR as GLint,
        );

        let vertices: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let (mut vao, mut vbo) = (0, 0);
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * size_of::<GLfloat>()) as GLsizeiptr,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let in_pos = CString::new("in_pos").unwrap();
            let pos_attrib = gl::GetAttribLocation(quad_program, in_pos.as_ptr());
            if pos_attrib >= 0 {
                gl::EnableVertexAttribArray(pos_attrib as GLuint);
                gl::VertexAttribPointer(
                    pos_attrib as GLuint,
                    2,
                    gl::FLOAT,
                    gl::FALSE,
                    2 * size_of::<GLfloat>() as GLsizei,
                    ptr::null(),
                );
            } else {
                println!("Warning: Vertex attribute 'in_pos' not found");
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        // Allocate minimal SSBOs; they will grow as needed when updated.
        let object_ssbo = create_ssbo::<GpuObject>(1, 1);
        let triangle_ssbo = create_ssbo::<GpuTriangle>(2, 1);
        let bvh_ssbo = create_ssbo::<crate::scene::bvh::GpuBvhNode>(3, 1);
        let tri_bvh_ssbo = create_ssbo::<crate::scene::tri_bvh::GpuTriBvhNode>(4, 1);

        Self {
            screen_width,
            screen_height,
            render_width,
            render_height,
            render_scale: 1.0,
            frame_number: 0,
            is_2d,
            use_fsr: false,
            sharpness: 0.0,
            ray_program,
            denoise_program,
            quad_program,
            blur_program,
            texture,
            ray_texture,
            depth_texture,
            normal_texture,
            capture_texture,
            vao,
            vbo,
            object_ssbo,
            triangle_ssbo,
            bvh_ssbo,
            tri_bvh_ssbo,
            uniforms,
            denoise_uniforms,
            prev_triangles: Vec::new(),
            prev_bvh: Vec::new(),
            prev_tri_bvh: Vec::new(),
        }
    }
    pub fn resize(&mut self, width: i32, height: i32) {
        self.screen_width = width;
        self.screen_height = height;
        self.render_width = (width as f32 * self.render_scale) as i32;
        self.render_height = (height as f32 * self.render_scale) as i32;

        unsafe {
            gl::Viewport(0, 0, width, height);

            resize_texture(
                self.texture,
                self.render_width,
                self.render_height,
                gl::RGBA32F as GLint,
                gl::RGBA,
                gl::FLOAT,
            );
            resize_texture(
                self.ray_texture,
                self.render_width,
                self.render_height,
                gl::RGBA32F as GLint,
                gl::RGBA,
                gl::FLOAT,
            );
            resize_texture(
                self.depth_texture,
                self.render_width,
                self.render_height,
                gl::R32F as GLint,
                gl::RED,
                gl::FLOAT,
            );
            resize_texture(
                self.normal_texture,
                self.render_width,
                self.render_height,
                gl::RGBA32F as GLint,
                gl::RGBA,
                gl::FLOAT,
            );
            resize_texture(
                self.capture_texture,
                width,
                height,
                gl::RGBA as GLint,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
            );
        }

        self.reset_frame();
    }

    pub fn set_render_scale(&mut self, scale: f32) {
        self.render_scale = scale.clamp(0.1, 1.0);
        self.render_width = (self.screen_width as f32 * self.render_scale) as i32;
        self.render_height = (self.screen_height as f32 * self.render_scale) as i32;
        unsafe {
            resize_texture(
                self.texture,
                self.render_width,
                self.render_height,
                gl::RGBA32F as GLint,
                gl::RGBA,
                gl::FLOAT,
            );
            resize_texture(
                self.ray_texture,
                self.render_width,
                self.render_height,
                gl::RGBA32F as GLint,
                gl::RGBA,
                gl::FLOAT,
            );
            resize_texture(
                self.depth_texture,
                self.render_width,
                self.render_height,
                gl::R32F as GLint,
                gl::RED,
                gl::FLOAT,
            );
            resize_texture(
                self.normal_texture,
                self.render_width,
                self.render_height,
                gl::RGBA32F as GLint,
                gl::RGBA,
                gl::FLOAT,
            );
        }
        self.reset_frame();
    }

    pub fn enable_fsr(&mut self, sharpness: f32) {
        self.use_fsr = true;
        self.sharpness = sharpness;
    }

    pub fn disable_fsr(&mut self) {
        self.use_fsr = false;
    }
    pub fn update_scene_data(
        &mut self,
        objects: &[GpuObject],
        triangles: &[GpuTriangle],
        bvh: &[crate::scene::bvh::GpuBvhNode],
        tri_bvh: &[crate::scene::tri_bvh::GpuTriBvhNode],
    ) {
        update_ssbo(self.object_ssbo, objects);
        if self.prev_triangles.len() != triangles.len()
            || bytemuck::cast_slice::<GpuTriangle, u8>(&self.prev_triangles)
                != bytemuck::cast_slice::<GpuTriangle, u8>(triangles)
        {
            update_ssbo(self.triangle_ssbo, triangles);
            self.prev_triangles = triangles.to_vec();
        }
        if self.prev_bvh.len() != bvh.len()
            || bytemuck::cast_slice::<crate::scene::bvh::GpuBvhNode, u8>(&self.prev_bvh)
                != bytemuck::cast_slice::<crate::scene::bvh::GpuBvhNode, u8>(bvh)
        {
            update_ssbo(self.bvh_ssbo, bvh);
            self.prev_bvh = bvh.to_vec();
        }
        if self.prev_tri_bvh.len() != tri_bvh.len()
            || bytemuck::cast_slice::<crate::scene::tri_bvh::GpuTriBvhNode, u8>(&self.prev_tri_bvh)
                != bytemuck::cast_slice::<crate::scene::tri_bvh::GpuTriBvhNode, u8>(tri_bvh)
        {
            update_ssbo(self.tri_bvh_ssbo, tri_bvh);
            self.prev_tri_bvh = tri_bvh.to_vec();
        }
        //      println!("SSBO update: objects {} bytes, triangles {} bytes",
        //     objects.len() * size_of::<GpuObject>(),
        //     triangles.len() * size_of::<GpuTriangle>());
    }

    pub fn render(&mut self, params: &RenderParams) {
        unsafe {
            // Drain any previous GL errors so we only log those from this frame
            let mut err = gl::GetError();
            while err != gl::NO_ERROR {
                err = gl::GetError();
            }

            // Clear the screen
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            // Raytrace pass
            gl::UseProgram(self.ray_program);

            // Bind SSBOs
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 1, self.object_ssbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 2, self.triangle_ssbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, self.bvh_ssbo);
            gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 4, self.tri_bvh_ssbo);

            // Set uniforms
            gl::Uniform3fv(self.uniforms.camera_pos, 1, params.camera_pos.as_ptr());
            gl::Uniform3fv(self.uniforms.camera_front, 1, params.camera_front.as_ptr());
            gl::Uniform3fv(self.uniforms.camera_up, 1, params.camera_up.as_ptr());
            gl::Uniform3fv(self.uniforms.camera_right, 1, params.camera_right.as_ptr());
            gl::Uniform3fv(self.uniforms.camera_velocity, 1, params.velocity.as_ptr());
            gl::Uniform1f(self.uniforms.fov, params.fov);
            gl::Uniform1i(self.uniforms.is_fisheye, params.is_fisheye);
            gl::Uniform3fv(
                self.uniforms.skycolor,
                1,
                [
                    params.skycolor[0] / 255.0,
                    params.skycolor[1] / 255.0,
                    params.skycolor[2] / 255.0,
                ]
                .as_ptr(),
            );
            let halton = |mut idx: i32, base: i32| -> f32 {
                let mut f = 1.0f32;
                let mut r = 0.0f32;
                while idx > 0 {
                    f /= base as f32;
                    r += f * (idx % base) as f32;
                    idx /= base;
                }
                r
            };
            let jitter_x = (halton(self.frame_number + 1, 2) - 0.5) / self.render_width as f32;
            let jitter_y = (halton(self.frame_number + 1, 3) - 0.5) / self.render_height as f32;
            gl::Uniform2fv(self.uniforms.taa_jitter, 1, [jitter_x, jitter_y].as_ptr());
            gl::Uniform1f(self.uniforms.current_time, params.current_time);
            gl::Uniform1i(self.uniforms.frame_number, self.frame_number);
            gl::Uniform1i(self.uniforms.num_objects, params.num_objects);

            // Bind texture for compute shader output
            let compute_output = if self.is_2d {
                self.texture
            } else {
                self.ray_texture
            };
            gl::BindImageTexture(
                0,
                compute_output,
                0,
                gl::FALSE,
                0,
                gl::WRITE_ONLY,
                gl::RGBA32F,
            );
            gl::BindImageTexture(
                1,
                self.depth_texture,
                0,
                gl::FALSE,
                0,
                gl::WRITE_ONLY,
                gl::R32F,
            );
            gl::BindImageTexture(
                2,
                self.normal_texture,
                0,
                gl::FALSE,
                0,
                gl::WRITE_ONLY,
                gl::RGBA32F,
            );

            // Dispatch compute shader
            gl::DispatchCompute(
                ((self.render_width + 15) / 16) as u32,
                ((self.render_height + 15) / 16) as u32,
                1,
            );

            // Memory barrier before presenting or denoising
            gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

            // Denoise pass for 3D rendering
            if !self.is_2d {
                gl::UseProgram(self.denoise_program);
                gl::Uniform2fv(
                    self.denoise_uniforms.taa_jitter,
                    1,
                    [jitter_x, jitter_y].as_ptr(),
                );
                gl::Uniform1i(self.denoise_uniforms.frame_number, self.frame_number);
                gl::BindImageTexture(
                    0,
                    self.ray_texture,
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_ONLY,
                    gl::RGBA32F,
                );
                gl::BindImageTexture(
                    1,
                    self.texture,
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_WRITE,
                    gl::RGBA32F,
                );
                gl::BindImageTexture(
                    2,
                    self.depth_texture,
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_ONLY,
                    gl::R32F,
                );
                gl::BindImageTexture(
                    3,
                    self.normal_texture,
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_ONLY,
                    gl::RGBA32F,
                );
                gl::DispatchCompute(
                    ((self.render_width + 15) / 16) as u32,
                    ((self.render_height + 15) / 16) as u32,
                    1,
                );

                gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
            }

            // Use quad shader
            gl::UseProgram(self.quad_program);

            // Bind texture for quad
            let tex_name = CString::new("screenTex").unwrap();
            let tex_loc = gl::GetUniformLocation(self.quad_program, tex_name.as_ptr());
            if tex_loc == -1 {
                println!("Warning: Uniform 'screenTex' not found");
            }
            gl::Uniform1i(tex_loc, 0);
            let size_loc = gl::GetUniformLocation(
                self.quad_program,
                CString::new("texSize").unwrap().as_ptr(),
            );
            if size_loc >= 0 {
                gl::Uniform2f(
                    size_loc,
                    self.render_width as f32,
                    self.render_height as f32,
                );
            }
            let sharp_loc = gl::GetUniformLocation(
                self.quad_program,
                CString::new("sharpness").unwrap().as_ptr(),
            );
            if sharp_loc >= 0 {
                let s = if self.use_fsr { self.sharpness } else { 0.0 };
                gl::Uniform1f(sharp_loc, s);
            }
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture);

            // Render quad without affecting depth buffer
            gl::BindVertexArray(self.vao);
            gl::DepthMask(gl::FALSE);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::DepthMask(gl::TRUE);

            // Unbind
            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::UseProgram(0);

            self.frame_number += 1;

            // Check for OpenGL errors (drain to avoid repeated logs)
            let mut err = gl::GetError();
            while err != gl::NO_ERROR {
                println!("OpenGL Error in render: {}", err);
                err = gl::GetError();
            }
        }
    }

    pub fn screen_dimensions(&self) -> (i32, i32) {
        (self.screen_width, self.screen_height)
    }

    pub fn blur_regions(&mut self, regions: &[(i32, i32, i32, i32)], feather: f32) {
        unsafe {
            gl::UseProgram(self.blur_program);
            let res_loc = gl::GetUniformLocation(
                self.blur_program,
                CString::new("resolution").unwrap().as_ptr(),
            );
            gl::Uniform2f(res_loc, self.screen_width as f32, self.screen_height as f32);
            let feat_loc = gl::GetUniformLocation(
                self.blur_program,
                CString::new("feather").unwrap().as_ptr(),
            );
            gl::Uniform1f(feat_loc, feather);
            let tex_loc = gl::GetUniformLocation(
                self.blur_program,
                CString::new("screenTex").unwrap().as_ptr(),
            );
            gl::Uniform1i(tex_loc, 0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.capture_texture);
            gl::BindVertexArray(self.vao);
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::SCISSOR_TEST);
            let region_loc =
                gl::GetUniformLocation(self.blur_program, CString::new("region").unwrap().as_ptr());
            for (x, y, w, h) in regions {
                gl::Uniform4f(region_loc, *x as f32, *y as f32, *w as f32, *h as f32);
                gl::Scissor(*x, *y, *w, *h);
                gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            }
            gl::Disable(gl::SCISSOR_TEST);
            gl::Enable(gl::DEPTH_TEST);
            gl::BindVertexArray(0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::UseProgram(0);
        }
    }

    pub fn capture_screen(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.capture_texture);
            gl::CopyTexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA,
                0,
                0,
                self.screen_width,
                self.screen_height,
                0,
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    pub fn reset_frame(&mut self) {
        self.frame_number = 0;
    }

    #[cfg(feature = "wgpu")]
    pub fn set_post_fx_uniforms(&mut self, _fx: crate::rendering::wgpu_renderer::PostFxUniforms) {}
}

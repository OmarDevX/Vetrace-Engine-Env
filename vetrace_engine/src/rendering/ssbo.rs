// ssbo.rs
use gl::types::*;
use std::mem::size_of;

pub fn create_ssbo<T>(binding_point: GLuint, max_items: usize) -> GLuint {
    let mut ssbo = 0;
    unsafe {
        gl::GenBuffers(1, &mut ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
        gl::BufferData(
            gl::SHADER_STORAGE_BUFFER,
            (max_items * size_of::<T>()) as isize,
            std::ptr::null(),
            gl::DYNAMIC_DRAW,
        );
        gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, binding_point, ssbo);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
    }
    ssbo
}

pub fn update_ssbo<T>(ssbo: GLuint, data: &[T]) {
    unsafe {
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, ssbo);
        let needed = (data.len() * size_of::<T>()) as isize;
        let mut current: GLint = 0;
        gl::GetBufferParameteriv(
            gl::SHADER_STORAGE_BUFFER,
            gl::BUFFER_SIZE,
            &mut current,
        );
        if needed > current as isize {
            gl::BufferData(gl::SHADER_STORAGE_BUFFER, needed, std::ptr::null(), gl::DYNAMIC_DRAW);
        }
        gl::BufferSubData(gl::SHADER_STORAGE_BUFFER, 0, needed, data.as_ptr() as *const _);
        gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
    }
}
pub fn create_texture(width: i32, height: i32) -> GLuint {
    let mut tex = 0;
    unsafe {
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA32F as GLint,
            width,
            height,
            0,
            gl::RGBA,
            gl::FLOAT,
            std::ptr::null(),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
    tex
}

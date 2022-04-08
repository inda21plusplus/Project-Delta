use std::mem;

use common::{Mat4, Vec3};

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        // This function just uses `width / height` to calculate the aspect ratio, so `aspect, 1.`
        // should effectively do what we want.
        let proj = Mat4::perspective_fov_rh_zo(self.fovy, self.aspect, 1., self.znear, self.zfar);
        proj * view
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            view_proj: Self::get_view_proj(camera),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = Self::get_view_proj(camera);
    }

    fn get_view_proj(camera: &Camera) -> [[f32; 4]; 4] {
        unsafe { mem::transmute(opengl_to_wgpu_matrix() * camera.build_view_projection_matrix()) }
    }
}

#[rustfmt::skip]
pub fn opengl_to_wgpu_matrix() -> Mat4 {
    Mat4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}

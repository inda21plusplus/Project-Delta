use std::mem;

use common::{Mat4, Vec3};

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        // This function just uses `width / height` to calculate the aspect ratio, so `aspect, 1.`
        // should effectively do what we want.
        let proj = Mat4::perspective_fov_rh_zo(self.fovy, aspect, 1., self.znear, self.zfar);
        proj * view
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    world_pos: [f32; 3],
    _padding: u32,
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_proj: unsafe {
                mem::transmute(
                    opengl_to_wgpu_matrix()
                        * Mat4::perspective_fov_rh_zo(30f32.to_radians(), 2., 1., 0.1, 1000.)
                        * Mat4::look_at_rh(Vec3::zero(), -Vec3::unit_z(), Vec3::unit_y()),
                )
            },
            world_pos: [0.; 3],
            _padding: 0,
        }
    }
}

impl CameraUniform {
    pub fn new(camera: &Camera, aspect: f32) -> Self {
        Self {
            view_proj: Self::get_view_proj(camera, aspect),
            world_pos: camera.eye.into_array(),
            _padding: 0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, aspect: f32) {
        self.view_proj = Self::get_view_proj(camera, aspect);
        self.world_pos = camera.eye.into_array();
    }

    fn get_view_proj(camera: &Camera, aspect: f32) -> [[f32; 4]; 4] {
        unsafe {
            mem::transmute(opengl_to_wgpu_matrix() * camera.build_view_projection_matrix(aspect))
        }
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

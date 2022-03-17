pub mod context;
pub mod error;
pub mod renderer;

pub use context::Context;
pub use renderer::Renderer;

pub type Vec2 = vek::vec::repr_c::Vec2<f32>;
pub type Vec3 = vek::vec::repr_c::Vec3<f32>;
pub type Vec4 = vek::vec::repr_c::Vec4<f32>;

pub type Quaternion = vek::quaternion::repr_c::Quaternion<f32>;

pub type Mat2 = vek::mat::repr_c::Mat2<f32>;
pub type Mat3 = vek::mat::repr_c::Mat3<f32>;
pub type Mat4 = vek::mat::repr_c::Mat4<f32>;

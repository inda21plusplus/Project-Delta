use thiserror::Error;

#[derive(Error, Debug)]
pub enum RenderingError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("error loading resource")]
    LoadError(#[from] LoadError),
    #[error("couldn't find an adapter with the necessary features")]
    NoAdapter,
    #[error("couldn't get a device from the adapter")]
    NoDevice(#[from] wgpu::RequestDeviceError),
    #[error("TODO: idk what to write here")]
    SurfaceError(#[from] wgpu::SurfaceError),
}

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("missing directory/file")]
    Missing,
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("failed loading obj file")]
    ObjError(#[from] tobj::LoadError),
    #[error("failed loading image file")]
    ImgError(#[from] image::error::ImageError),
}

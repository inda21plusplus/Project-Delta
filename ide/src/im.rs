use std::path::Path;

use image::ImageError;

pub fn get_logo<P: AsRef<Path>>(file_name: P) -> Result<(u32, u32, Vec<u8>), ImageError> {
    let img = image::open(file_name)?;
    let rgba = img.into_rgba8();
    let (w, h) = rgba.dimensions();
    let content = rgba.into_raw();

    Ok((w, h, content))
}

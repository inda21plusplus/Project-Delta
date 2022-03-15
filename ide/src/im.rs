use std::{
    fs::{self, File},
    io::Read, error::Error,
};

pub fn get_logo(file_name: String) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let mut bff = String::new();

    let mut file = File::open(&file_name).expect("Unable to open file");
    file.read_to_string(&mut bff).expect("Unable to read");
    let mut lines = bff.lines();
    lines.next();

    // same format as ppm but with an added alpha channel
    let (width_str, height_str) = lines.next().ok_or("Failed to read line")?.split_once(' ').ok_or("failed to split")?;
    let width: usize = width_str.parse()?;
    let height: usize = height_str.parse()?;
    let mut content: Vec<u8> = Vec::with_capacity(width * height * 4);
    for _ in 0..height {
        for _ in 0..width {
            let items : Vec<u8> = lines.next().ok_or("Failed to read line")?.splitn(4, ' ').flat_map(|x| x.parse::<u8>()).collect();
            content.push(items[1]);
            content.push(items[2]);
            content.push(items[0]);
            content.push(items[3]);
        }
    }

    Ok((width as u32, height as u32, content))
}

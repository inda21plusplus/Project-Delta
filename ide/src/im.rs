use std::{
    fs::{self, File},
    io::Read,
};

pub fn get_logo(file_name: String) -> (u32, u32, Vec<u8>) {
    let mut bff = String::new();

    let mut file = File::open(&file_name).expect("Unable to open file");
    file.read_to_string(&mut bff).expect("Unable to read");
    let mut lines = bff.lines();
    lines.next();
    let (width_str, height_str) = lines.next().unwrap().split_once(' ').unwrap();
    let width: usize = width_str.parse().unwrap();
    let height: usize = height_str.parse().unwrap();
    let mut content: Vec<u8> = Vec::with_capacity(width * height * 4);
    for _ in 0..height {
        for _ in 0..width {
            let mut line = lines.next().unwrap().splitn(4, ' ');
            let r: u8 = line.next().unwrap().parse().unwrap();
            let g: u8 = line.next().unwrap().parse().unwrap();
            let b: u8 = line.next().unwrap().parse().unwrap();
            let a: u8 = line.next().unwrap().parse().unwrap();
            content.push(g);
            content.push(b);
            content.push(r);
            content.push(a);
        }
    }

    (width as u32, height as u32, content)
}

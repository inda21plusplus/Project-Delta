use std::{
    fs::{self, File},
    io::Read,
};

pub fn get_logo(file_name: String) -> Vec<u8> {
    //let mut file_content = Vec::new();

    //let mut file = File::open(&file_name).expect("Unable to open file");
    //file.read_to_end(&mut file_content).expect("Unable to read");
    let f2: Vec<u8> = fs::read_to_string(file_name)
        .unwrap()
        .split(" ")
        .map(|num| num.parse::<u8>().unwrap())
        .collect();
    println!("{:?}", f2);
    println!("{}", f2.len());
    f2
}

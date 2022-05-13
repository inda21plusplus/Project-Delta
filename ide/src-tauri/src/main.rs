use std::process::Command;
use std::{
    io::{Read, Write},
    ops::DerefMut,
    path::PathBuf,
    process::{Child, Stdio},
    sync::{Arc, Mutex},
};

use tauri::{api::dialog::FileDialogBuilder, Manager};
struct MyState(Arc<Mutex<Option<PathBuf>>>, Mutex<Option<Child>>);

fn main() {
    tauri::Builder::default()
        .setup(|app| Ok(()))
        .manage(MyState(Arc::new(Mutex::new(None)), Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            select_game,
            play_game,
            stop_game,
            pause_game,
            unpause_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn select_game(state: tauri::State<'_, MyState>) {
    let state_store = state.0.clone();
    FileDialogBuilder::default().pick_folder(move |folder| {
        let mut state_lock = state_store.lock().unwrap();
        *state_lock = folder;

        println!("{:?}", state_lock);
    });
}

#[tauri::command]
fn play_game(state: tauri::State<'_, MyState>) {
    let path = state.0.lock().unwrap();
    let path = path.clone();
    println!("{:?}", path);
    let child = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("ide")
        .current_dir(path.unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("ls command failed to start");

    let mut state_lock = state.1.lock().unwrap();
    *state_lock = Some(child);
}

#[tauri::command]
fn stop_game(state: tauri::State<'_, MyState>) {
    let mut state_lock = state.1.lock().unwrap();

    if let Some(child) = state_lock.deref_mut() {
        child.kill().unwrap();

        let store = state_lock.deref_mut();
        *store = None;
    }
}

#[tauri::command]
fn pause_game(state: tauri::State<'_, MyState>) {
    // let path = state.0.lock().unwrap();
    // let path = path.clone();
    // println!("{:?}", path);
    // let child = Command::new("cargo")
    //   .arg("run")
    //   .arg("-p")
    //   .arg("ide")
    //   .current_dir(path.unwrap())
    //   .spawn()
    //   .expect("ls command failed to start");

    //   let mut state_lock = state.1.lock().unwrap();
    //   *state_lock = Some(child);
}

#[tauri::command]
fn unpause_game(state: tauri::State<'_, MyState>) {
    // let path = state.0.lock().unwrap();
    // let path = path.clone();
    // println!("{:?}", path);
    // let child = Command::new("cargo")
    //   .arg("run")
    //   .arg("-p")
    //   .arg("ide")
    //   .current_dir(path.unwrap())
    //   .spawn()
    //   .expect("ls command failed to start");

    //   let mut state_lock = state.1.lock().unwrap();
    //   *state_lock = Some(child);
}

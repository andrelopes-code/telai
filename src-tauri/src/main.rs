// Release builds are GUI apps; without this, Windows also opens a console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod browser_audio;
mod commands;
mod live;
mod text;
mod tray;

fn main() {
    app::run();
}

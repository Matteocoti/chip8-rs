mod actions;
mod app;
mod browser;
mod chip8;
mod chip8_tui;
mod config_file;
mod constants;
mod menu;
mod settings;
use crate::app::App;

fn main() {
    let mut app = App::new();
    let _ = app.run();
}

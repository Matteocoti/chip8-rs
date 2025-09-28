mod actions;
mod app;
mod audio;
mod browser;
mod chip8;
mod chip8_tui;
mod config_file;
mod constants;
mod menu;
mod performance_metrics;
mod settings;
use crate::app::App;

fn main() {
    let mut app = App::new();
    let _ = app.run();
}

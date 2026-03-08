mod actions;
mod app;
mod audio;
mod chip8;
mod chip8_tui;
mod component;
mod config_file;
mod config_manager;
mod constants;
mod file_browser;
mod menu;
mod performance_metrics;
mod rom_history;
mod settings;
mod split_view_component;
use crate::app::App;

fn main() {
    let mut app = App::new();
    let _ = app.run();
}

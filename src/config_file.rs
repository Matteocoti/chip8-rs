use std::path::PathBuf;

/// Returns the path to the application persistent data
///
/// The function retrieves the path from the system home directory.
/// If the home directory cannot be defined, None is returned.
fn get_config_path() -> Option<PathBuf> {
    if let Some(home_dir) = home::home_dir() {
        let config_path = home_dir.join(".chip8_tui");
        Some(config_path)
    } else {
        None
    }
}

pub fn get_settings_file_path() -> Option<PathBuf> {
    let config_path = get_config_path();

    config_path.map(|path| path.join("settings.toml"))
}

pub fn get_rom_path() -> Option<PathBuf> {
    let config_path = get_config_path();
    config_path.map(|path| path.join("roms.toml"))
}

fn get_saved_data_path() -> Option<PathBuf> {
    let config_path = get_config_path();

    config_path.map(|path| path.join("saved_data"))
}

pub fn get_rom_saved_data_path(rom_name: &str) -> Option<PathBuf> {
    let save_data_path = get_saved_data_path();

    save_data_path.map(|path| path.join(rom_name))
}

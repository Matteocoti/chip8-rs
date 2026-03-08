use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigManager {
    #[allow(dead_code)]
    pub base_path: PathBuf,
    pub emulator_settings_path: PathBuf,
    pub key_bindings_path: PathBuf,
    pub rom_history_path: PathBuf,
    pub log_path: PathBuf,
}

impl ConfigManager {
    /// Creates a new ConfigManager, defining and ensuring the existence of config directories.
    pub fn new() -> Self {
        let base_path = Self::get_or_create_base_path();
        let emulator_settings_path = base_path.join("emulator_settings.toml");
        let key_bindings_path = base_path.join("key_bindings.toml");
        let rom_history_path = base_path.join("rom_history.toml");
        let log_path = base_path.join("chip8.log");

        Self {
            base_path,
            emulator_settings_path,
            key_bindings_path,
            rom_history_path,
            log_path,
        }
    }

    /// Determines the base path for configuration files and creates it if it doesn't exist.
    fn get_or_create_base_path() -> PathBuf {
        if let Some(home_dir) = home::home_dir() {
            let config_path = home_dir.join(".chip8_tui");
            if !config_path.exists()
                && let Err(e) = fs::create_dir_all(&config_path)
            {
                eprintln!("Failed to create config directory: {}", e);
                // Fallback to current directory
                return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            }
            config_path
        } else {
            // Fallback for systems without a home directory
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

use crate::audio::AudioHandler;
use crate::chip8::*;
use crate::component::{Action, Component, Transition};
use crate::config_file::get_rom_saved_data_path;
use crate::config_manager::ConfigManager;
use crate::rom_history::RomHistory;
use crate::settings::{EmulatorSettings, KeyBindings};
use chrono::Utc;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};
use std::fs;
use std::{collections::{HashMap, HashSet}, path::PathBuf};

pub struct Chip8TUI {
    core: Chip8,
    rom: Option<PathBuf>,
    rom_name: String,
    keymap: HashMap<char, u8>,
    #[allow(dead_code)]
    max_delta_time: u16,
    sound_hdrl: Option<AudioHandler>,
    display_string_cache: String,
    step_mode: bool,
    step: bool,
    start_frequency: u16,
    current_frequency: u16,
    frequency_step: u16,
    held_keys: HashSet<u8>,
    pending_notification: Option<String>,
}

impl Chip8TUI {
    pub fn new(rom: &PathBuf) -> Self {
        let config = ConfigManager::new();
        let key_bindings = KeyBindings::load(&config.key_bindings_path);
        let emulator_settings = EmulatorSettings::load(&config.emulator_settings_path);

        let frequency = emulator_settings.get_frequency();
        let max_delta_time = emulator_settings.get_max_delta_time();
        let frequency_step = (frequency / 4).max(1);

        let mut keymap = HashMap::new();
        for (chip8_key, &qwerty_char) in key_bindings.get_keyboard().iter().enumerate() {
            keymap.insert(qwerty_char, chip8_key as u8);
        }

        let sound_hdrl = AudioHandler::new();

        let mut core = Chip8::new();
        core.set_frequency(frequency);
        core.set_max_delta_time(max_delta_time);

        let mut tui = Self {
            core,
            rom: None,
            rom_name: String::new(),
            keymap,
            max_delta_time,
            sound_hdrl,
            display_string_cache: String::with_capacity(64 * 32 + 31),
            step_mode: false,
            step: true,
            start_frequency: frequency,
            current_frequency: frequency,
            frequency_step,
            held_keys: HashSet::new(),
            pending_notification: None,
        };

        if let Err(msg) = tui.load_rom(rom) {
            tui.pending_notification = Some(msg);
        }

        tui
    }

    fn play_sound(&mut self) {
        if let Some(ref sound_hdrl) = self.sound_hdrl {
            sound_hdrl.play();
        }
    }
    fn stop_sound(&mut self) {
        if let Some(ref sound_hdrl) = self.sound_hdrl {
            sound_hdrl.pause();
        }
    }

    pub fn load_rom(&mut self, rom_path: &PathBuf) -> bool {
        let rom_rd_res = std::fs::read(&rom_path);

        if let Ok(rom_data) = rom_rd_res {
            if self.core.load_rom(rom_data) {
                self.rom_name = rom_path.file_name().unwrap().to_string_lossy().into_owned();
                self.rom = Some(rom_path.clone());
                return true;
            }
        }
        false
    }

    // pub fn config(&mut self, settings: &Settings) {
    //     self.keymap.clear();
    //     self.max_delta_time = settings.get_max_delta_time();
    //     self.start_frequency = settings.get_frequency();
    //     self.current_frequency = self.start_frequency;
    //     self.frequency_step = self.start_frequency / 4;
    //     self.core.set_frequency(self.current_frequency);
    //     self.core.set_max_delta_time(self.max_delta_time);
    //     let keymap = settings.get_key_mappings();
    //
    //     for (chip8_key, key_char) in keymap.iter().enumerate() {
    //         self.keymap.insert(*key_char, chip8_key as u8);
    //     }
    // }

    fn save_state(&mut self, file_name: &str) -> Action {
        let name = self.rom_name.as_ref();

        if let Some(save_path) = get_rom_saved_data_path(name) {
            let save_file_name = save_path.join(file_name);
            if let Some(parent_dir) = save_file_name.parent() {
                let _ = fs::create_dir_all(parent_dir);
            }
            match self.core.save_state(&save_file_name) {
                Ok(_) => Action::Render,
                Err(_) => Action::Nope,
            }
        } else {
            Action::Nope
        }
    }

    fn quick_save_state(&mut self) -> Action {
        let ts = Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let file_name = format!("{}_{}.sav", &self.rom_name, ts);
        match self.save_state(&file_name) {
            Action::Render => Action::Notify(format!("Saved: {file_name}")),
            _ => Action::Notify("Save failed".to_string()),
        }
    }

    fn find_latest_save_file(&self, rom_name: &str) -> Option<PathBuf> {
        let rom_save_data_path = get_rom_saved_data_path(rom_name)?;

        let path = rom_save_data_path;
        let entries = fs::read_dir(&path).ok()?;
        let mut save_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.ends_with(".sav") {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            save_files.push((path, modified));
                        }
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        save_files.sort_by(|a, b| b.1.cmp(&a.1));

        save_files.first().map(|(path, _)| path.clone())
    }

    fn quick_load_state(&mut self) -> Action {
        if self.rom.is_none() {
            return Action::Nope;
        }

        let saved_data = self.find_latest_save_file(&self.rom_name);

        if let Some(data) = saved_data {
            match self.core.load_state(&data) {
                Ok(_) => Action::Notify(format!(
                    "Loaded: {}",
                    data.file_name().unwrap_or_default().to_string_lossy()
                )),
                Err(e) => Action::Notify(format!("Load failed: {e}")),
            }
        } else {
            Action::Notify("No save file found".to_string())
        }
    }

    fn reset_rom(&mut self) -> Action {
        if let Some(rom_path) = self.rom.clone() {
            self.core.reset();
            if let Err(msg) = self.load_rom(&rom_path) {
                return Action::Notify(msg);
            }
        }
        Action::Nope
    }

    fn inc_frequency(&mut self) {
        self.current_frequency += self.frequency_step;
        self.core.set_frequency(self.current_frequency);
    }

    fn dec_frequency(&mut self) {
        if self.current_frequency > self.frequency_step {
            self.current_frequency -= self.frequency_step
        }
        self.core.set_frequency(self.current_frequency);
    }

    fn reload_frequency(&mut self) {
        self.current_frequency = self.start_frequency;
        self.core.set_frequency(self.current_frequency);
    }
}

impl Component for Chip8TUI {
    fn on_entry(&mut self) -> Action {
        if let Some(rom_path) = &self.rom.clone() {
            let config = ConfigManager::new();
            let mut history = RomHistory::load(&config.rom_history_path);
            history.register_rom(rom_path.clone());
            let _ = history.save_to_file(&config.rom_history_path);
        }
        if let Some(msg) = self.pending_notification.take() {
            return Action::Notify(msg);
        }
        Action::Nope
    }

    fn handle_key_event(&mut self, event: KeyEvent) -> Action {
        match event.code {
            KeyCode::F(1) => self.inc_frequency(),
            KeyCode::F(2) => self.dec_frequency(),
            KeyCode::F(3) => self.reload_frequency(),
            KeyCode::F(4) => self.reset_rom(),
            KeyCode::F(5) => return self.quick_save_state(),
            KeyCode::F(6) => return self.quick_load_state(),
            KeyCode::Enter => self.step_mode = !self.step_mode,
            KeyCode::Char('n') => self.step = true,
            KeyCode::Char(key) => {
                if let Some(&chip8_key) = self.keymap.get(&key) {
                    self.held_keys.insert(chip8_key);
                    self.core.press_key(chip8_key);
                }
            }
            KeyCode::Esc => {
                self.stop_sound();
                return Action::Transition(Transition::Pop);
            }
            _ => (),
        }
        Action::Nope
    }

    fn handle_key_release(&mut self, event: KeyEvent) -> Action {
        if let KeyCode::Char(key) = event.code {
            if let Some(chip8_key) = self.keymap.get(&key) {
                self.held_keys.remove(chip8_key);
            }
        }
        Action::Nope
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.display_string_cache.clear();

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)].as_ref())
            .split(area);

        let frame_data = self.core.get_frame_buffer();

        let mut rows = frame_data.chunks_exact(64).peekable();
        while let Some(row_slice) = rows.next() {
            for &pixel_on in row_slice {
                self.display_string_cache
                    .push(if pixel_on { '█' } else { ' ' });
            }
            if rows.peek().is_some() {
                self.display_string_cache.push('\n');
            }
        }
        let display = Paragraph::new(self.display_string_cache.as_str())
            .block(Block::default().title("Display").borders(Borders::ALL));

        f.render_widget(display, chunks[0]);

        let state_string = self.core.get_state().to_string();

        let paragraph = Paragraph::new(state_string);

        f.render_widget(paragraph, chunks[1]);
    }

    fn update(&mut self) -> Action {
        if self.step_mode {
            if self.step {
                self.step = false;
            } else {
                return Action::Nope;
            }
        }
        // Re-apply any held keys before ticking so they persist across frames
        for &key in &self.held_keys {
            self.core.press_key(key);
        }
        let mut action = Action::Nope;
        match self.core.tick() {
            Ok(events) => {
                for evt in events {
                    match evt {
                        EmulationEvent::ScreenUpdated => {
                            action = Action::Render;
                        }
                        EmulationEvent::SoundStarted => {
                            self.play_sound();
                        }
                        EmulationEvent::SoundStopped => {
                            self.stop_sound();
                        }
                    }
                }
            }
            Err(e) => return Action::Notify(format!("Emulation error: {e}")),
        }
        action
    }
}

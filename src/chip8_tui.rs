use crate::audio::AudioHandler;
use crate::chip8::cpu::EmulationEvent;
use crate::chip8::*;
use crate::config_file::get_rom_saved_data_path;
use crate::{actions::Action, settings::Settings};
use chrono::Utc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    widgets::{Block, Borders, Paragraph},
};
use std::fs;
use std::{collections::HashMap, path::PathBuf};

pub struct Chip8TUI {
    core: Chip8,
    rom: Option<PathBuf>,
    rom_name: String,
    keymap: HashMap<char, u8>,
    max_delta_time: u16,
    sound_hdrl: Option<AudioHandler>,
    display_string_cache: String,
    step_mode: bool,
    step: bool,
}

impl Chip8TUI {
    pub fn new() -> Self {
        let sound_hdrl = AudioHandler::new();
        Self {
            core: Chip8::new(),
            rom: None,
            rom_name: String::new(),
            keymap: HashMap::new(),
            max_delta_time: 30, // 30ms
            sound_hdrl,
            display_string_cache: String::with_capacity(64 * 32 + 31),
            step_mode: false,
            step: true,
        }
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

    pub fn update(&mut self) -> Action {
        if self.step_mode {
            if self.step {
                self.step = false;
            } else {
                return Action::Nope;
            }
        }
        let emu_result = self.core.tick();
        let mut action = Action::Nope;
        if let Ok(events) = emu_result {
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
        action
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

    pub fn config(&mut self, settings: &Settings) {
        self.keymap.clear();
        self.max_delta_time = settings.get_max_delta_time();
        self.core.set_frequency(settings.get_frequency());
        self.core.set_max_delta_time(self.max_delta_time);
        let keymap = settings.get_key_mappings();

        for (chip8_key, key_char) in keymap.iter().enumerate() {
            self.keymap.insert(*key_char, chip8_key as u8);
        }
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> Action {
        match event.code {
            KeyCode::F(4) => self.reset_rom(),
            KeyCode::F(5) => return self.quick_save_state(),
            KeyCode::F(6) => return self.quick_load_state(),
            KeyCode::Enter => self.step_mode = !self.step_mode,
            KeyCode::Char('n') => self.step = true,
            KeyCode::Char(key) => {
                if let Some(chip8_key) = self.keymap.get(&key) {
                    self.core.press_key(*chip8_key);
                }
            }
            KeyCode::Esc => {
                self.stop_sound();
                return Action::GoToMenu;
            }
            _ => (),
        }
        Action::Nope
    }

    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        self.display_string_cache.clear();

        let _ = terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)].as_ref())
                .split(f.area());

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
        });
    }

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
        let ts = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let file_name = format!("{}_{}.sav", &self.rom_name, ts);
        self.save_state(&file_name)
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
                Ok(_) => {
                    self.step_mode = true;
                    Action::Render
                }
                Err(_) => Action::Nope,
            }
        } else {
            Action::Nope
        }
    }

    fn reset_rom(&mut self) {
        if let Some(rom_path) = self.rom.clone() {
            self.core.reset();
            let _ = self.load_rom(&rom_path);
        }
    }
}

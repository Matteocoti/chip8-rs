use crate::chip8::Chip8;
use crate::{actions::Action, settings::Settings};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use std::{collections::HashMap, path::PathBuf};

pub struct Chip8TUI {
    core: Chip8,
    keymap: HashMap<char, u8>,
    freq: u16,
}

impl Chip8TUI {
    pub fn new() -> Self {
        Self {
            core: Chip8::new(),
            keymap: HashMap::new(),
            freq: 60,
        }
    }

    pub fn update(&mut self) -> Action {
        let emu_result = self.core.tick();
        self.core.reset_keyboard();
        let mut action = Action::Nope;
        if let Ok(update) = emu_result {
            if update {
                action = Action::Render;
            }
        }
        action
    }

    pub fn load_rom(&mut self, rom_path: &PathBuf) -> bool {
        let rom_rd_res = std::fs::read(rom_path);
        if let Ok(rom_data) = rom_rd_res {
            if self.core.load_rom(rom_data) {
                return true;
            }
        }
        false
    }

    pub fn config(&mut self, settings: &Settings) {
        self.keymap.clear();
        self.freq = settings.get_frequency();
        let keymap = settings.get_key_mappings();

        for (chip8_key, key_char) in keymap.iter().enumerate() {
            self.keymap.insert(*key_char, chip8_key as u8);
        }
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> Action {
        match event.code {
            KeyCode::Char(key) => {
                if let Some(chip8_key) = self.keymap.get(&key) {
                    self.core.press_key(*chip8_key);
                }
            }
            KeyCode::Esc => return Action::GoToMenu,
            _ => (),
        }
        Action::Nope
    }

    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _ = terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)].as_ref())
                .split(f.area());

            let frame_data = self.core.get_frame_buffer();

            let lines: Vec<Line> = frame_data
                .chunks_exact(64)
                .map(|row_slice| {
                    let row_string = row_slice
                        .iter()
                        .map(|&pixel_on| if pixel_on { '█' } else { ' ' })
                        .collect::<String>();

                    Line::from(row_string)
                })
                .collect();
            let display = Paragraph::new(lines)
                .block(Block::default().title("Display").borders(Borders::ALL));

            f.render_widget(display, chunks[0]);

            let state_string = self.core.get_state().to_string();

            let paragraph = Paragraph::new(state_string);

            f.render_widget(paragraph, chunks[1]);
        });
    }
}

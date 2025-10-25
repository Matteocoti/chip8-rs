use crate::chip8_tui::Chip8TUI;
use crate::component::{Action, Component, Transition};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
struct RomFileData {
    path: String,
    name: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RomHistory {
    roms: Vec<RomFileData>,
    #[serde(skip)]
    state: ListState,
    file_path: PathBuf,
}

impl RomHistory {
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.roms.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.roms.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn register_rom(&mut self, rom_path: PathBuf) {
        let name = rom_path.file_name().unwrap().to_string_lossy().into_owned();
        let path = rom_path.parent().unwrap().to_string_lossy().into_owned();

        let rom = RomFileData { path, name };

        if !self.roms.contains(&rom) {
            self.roms.push(rom);
        }
    }

    pub fn render_footer(&self) -> Line {
        // Definisci gli stili come prima
        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let text_style = Style::default().fg(Color::Gray);

        Line::from(vec![
            Span::styled("[↑↓]", key_style),
            Span::styled(" Nav", text_style),
            Span::raw(" | "),
            Span::styled("[Enter]", key_style),
            Span::styled(" Open", text_style),
        ])
    }

    fn load_to_file(path: &PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let history = toml::from_str(&content).expect("Failed to deserialize ROM history");
        Ok(history)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> io::Result<()> {
        let content = toml::to_string_pretty(self).expect("Failed to serialize ROM history");
        fs::write(path, content)
    }

    pub fn load(path: &PathBuf) -> Self {
        if let Ok(mut history) = Self::load_to_file(path) {
            history.state = ListState::default();
            history.state.select(Some(0));
            history.file_path = path.clone();
            history
        } else {
            let mut history = Self::default();
            history.file_path = path.clone();
            history
        }
    }
}

impl Component for RomHistory {
    fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
        match evt.code {
            KeyCode::Up => {
                self.previous();
                Action::Render
            }
            KeyCode::Down => {
                self.next();
                Action::Render
            }
            KeyCode::Enter => {
                if let Some(selected_index) = self.state.selected() {
                    if selected_index < self.roms.len() {
                        let rom = &self.roms[selected_index];
                        let rom_path = PathBuf::from(&rom.path).join(&rom.name);
                        return Action::Transition(Transition::Switch(Box::new(Chip8TUI::new(
                            &rom_path,
                        ))));
                    }
                }
                Action::Nope
            }
            _ => Action::Nope,
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let rom_items: Vec<ListItem> = self
            .roms
            .iter()
            .map(|rom| {
                let display_text = format!("{} (path: {})", rom.name, rom.path);
                ListItem::new(display_text)
            })
            .collect();
        let roms_list = List::new(rom_items)
            .block(Block::default().borders(Borders::ALL).title("ROM History"))
            .highlight_symbol(">> ");

        f.render_stateful_widget(roms_list, area, &mut self.state);
    }

    fn on_exit(&mut self) -> Action {
        let _ = self.save_to_file(&self.file_path);
        Action::Nope
    }
}

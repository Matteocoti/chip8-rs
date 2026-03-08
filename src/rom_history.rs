use crate::chip8_tui::Chip8TUI;
use crate::component::{Action, Component, Transition};
use crate::config_manager::ConfigManager;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
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
    #[serde(skip)]
    config: ConfigManager,
}

impl RomHistory {
    fn next(&mut self) {
        if self.roms.is_empty() {
            return;
        }
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
        if self.roms.is_empty() {
            return;
        }
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

    #[allow(dead_code)]
    pub fn register_rom(&mut self, rom_path: PathBuf) {
        let name = match rom_path.file_name() {
            Some(n) => n.to_string_lossy().into_owned(),
            None => return,
        };
        let path = rom_path
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        let rom = RomFileData { path, name };

        if !self.roms.contains(&rom) {
            self.roms.push(rom);
        }
    }

    #[allow(dead_code)]
    pub fn render_footer(&self) -> Line {
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

    fn load_from_file(path: &PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let history =
            toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(history)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> io::Result<()> {
        let content = toml::to_string_pretty(self).expect("Failed to serialize ROM history");
        fs::write(path, content)
    }

    pub fn load(path: &PathBuf, config: ConfigManager) -> Self {
        if let Ok(mut history) = Self::load_from_file(path) {
            history.state = ListState::default();
            history.state.select(Some(0));
            history.file_path = path.clone();
            history.config = config;
            history
        } else {
            Self {
                file_path: path.clone(),
                config,
                ..Self::default()
            }
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
                            self.config.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rom(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    #[test]
    fn register_rom_adds_entry() {
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        assert_eq!(history.roms.len(), 1);
        assert_eq!(history.roms[0].name, "pong.ch8");
    }

    #[test]
    fn register_rom_deduplicates() {
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        history.register_rom(rom("/roms/pong.ch8"));
        assert_eq!(history.roms.len(), 1);
    }

    #[test]
    fn register_different_roms_adds_both() {
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        history.register_rom(rom("/roms/tetris.ch8"));
        assert_eq!(history.roms.len(), 2);
    }

    #[test]
    fn next_advances_selection() {
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        history.register_rom(rom("/roms/tetris.ch8"));
        history.state.select(Some(0));
        history.next();
        assert_eq!(history.state.selected(), Some(1));
    }

    #[test]
    fn next_wraps_from_last_to_first() {
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        history.register_rom(rom("/roms/tetris.ch8"));
        history.state.select(Some(1));
        history.next();
        assert_eq!(history.state.selected(), Some(0));
    }

    #[test]
    fn previous_wraps_from_first_to_last() {
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        history.register_rom(rom("/roms/tetris.ch8"));
        history.state.select(Some(0));
        history.previous();
        assert_eq!(history.state.selected(), Some(1));
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = std::env::temp_dir().join("chip8_test_rom_history.toml");
        let mut history = RomHistory::default();
        history.register_rom(rom("/roms/pong.ch8"));
        history.register_rom(rom("/roms/tetris.ch8"));
        history.save_to_file(&path).unwrap();

        let loaded = RomHistory::load(&path, ConfigManager::default());
        assert_eq!(loaded.roms.len(), 2);
        assert_eq!(loaded.roms[0].name, "pong.ch8");
        assert_eq!(loaded.roms[1].name, "tetris.ch8");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_from_nonexistent_file_returns_default() {
        let path = PathBuf::from("/nonexistent/path/chip8_history.toml");
        let history = RomHistory::load(&path, ConfigManager::default());
        assert_eq!(history.roms.len(), 0);
    }

    #[test]
    fn load_from_corrupt_toml_returns_default() {
        let path = std::env::temp_dir().join("chip8_test_corrupt_history.toml");
        std::fs::write(&path, b"this is not valid toml [[[").unwrap();
        let history = RomHistory::load(&path, ConfigManager::default());
        assert_eq!(history.roms.len(), 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn register_rom_with_rootlike_path_does_not_panic() {
        let mut history = RomHistory::default();
        // A path with no parent (e.g. just a filename with no directory)
        // should be handled gracefully, not panic.
        history.register_rom(PathBuf::from("pong.ch8"));
        assert_eq!(history.roms.len(), 1);
        assert_eq!(history.roms[0].name, "pong.ch8");
    }
}

use std::{fs, io, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use serde::{Deserialize, Serialize};

use crate::component::{Action, Component};

/// A component for managing and displaying CHIP-8 keybindings.
///
/// It allows for navigating a list of key mappings, entering an "edit mode"
/// to change a specific key, and rendering the list to the UI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyBindings {
    /// A vector where the index represents the CHIP-8 key (0x0-0xF)
    /// and the value is the character mapped to it.
    keyboard: Vec<char>,
    /// The state of the list in the UI, tracking the selected item.
    #[serde(skip)]
    state: ListState,
    /// The index of the item currently being edited. `None` if not in edit mode.
    #[serde(skip)]
    editing_index: Option<usize>,
    /// Path to the file where key bindings are persisted.
    #[serde(skip)]
    path: PathBuf,
}

impl KeyBindings {
    /// Moves the selection to the next item in the list, wrapping around.
    fn next(&mut self) {
        let i = self
            .state
            .selected()
            .map_or(0, |i| (i + 1) % self.keyboard.len());
        self.state.select(Some(i));
    }

    /// Moves the selection to the previous item in the list, wrapping around.
    fn previous(&mut self) {
        let i = self.state.selected().map_or(0, |i| {
            if i == 0 {
                self.keyboard.len() - 1
            } else {
                i - 1
            }
        });
        self.state.select(Some(i));
    }

    pub fn get_keyboard(&self) -> &[char] {
        &self.keyboard
    }

    fn load_from_file(path: &PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let bindings: Self = toml::from_str(&content).expect("Failed to deserialize key bindings");
        Ok(bindings)
    }

    fn save_to_file(&self, path: &PathBuf) -> io::Result<()> {
        let content = toml::to_string_pretty(self).expect("Failed to serialize key bindings");
        fs::write(path, content)
    }

    pub fn load(path: &PathBuf) -> Self {
        if let Ok(mut data) = Self::load_from_file(path) {
            data.state = ListState::default();
            data.state.select(Some(0));
            data.editing_index = None;
            data.path = path.clone();
            data
        } else {
            let mut default = Self::default();
            default.path = path.clone();
            default
        }
    }
}

impl Default for KeyBindings {
    /// Creates a new `KeyBindings` component with a default layout.
    fn default() -> Self {
        let mut state = ListState::default();
        state.select(Some(0)); // Select the first item by default

        Self {
            keyboard: vec![
                '1', '2', '3', '4', 'q', 'w', 'e', 'r', 'a', 's', 'd', 'f', 'z', 'x', 'c', 'v',
            ],
            state,
            editing_index: None,
            path: PathBuf::new(),
        }
    }
}

impl Component for KeyBindings {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if let Some(index) = self.editing_index {
            match key.code {
                KeyCode::Char(c) => {
                    self.keyboard[index] = c;
                    self.editing_index = None;
                    Action::Render
                }
                KeyCode::Esc => {
                    self.editing_index = None;
                    Action::Render
                }
                _ => Action::Nope,
            }
        } else {
            match key.code {
                KeyCode::Up => {
                    self.previous();
                    Action::Render
                }
                KeyCode::Down => {
                    self.next();
                    Action::Render
                }
                KeyCode::Enter => {
                    self.editing_index = self.state.selected();
                    Action::Render
                }
                _ => Action::Nope,
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .keyboard
            .iter()
            .enumerate()
            .map(|(chip8_key_index, qwerty_key)| {
                // The index 0-15 is also the CHIP-8 key value 0x0-0xF
                let line = format!("0x{:X} -> '{}'", chip8_key_index, qwerty_key);
                ListItem::new(line)
            })
            .collect();

        let mut widget = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Impostazioni Tastiera"),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        // If editing, change highlight style to indicate edit mode
        if self.editing_index.is_some() {
            widget = widget.highlight_style(Style::default().fg(Color::Black).bg(Color::White));
        }

        frame.render_stateful_widget(widget, area, &mut self.state);
    }

    fn on_exit(&mut self) -> Action {
        if let Err(e) = self.save_to_file(&self.path) {
            eprintln!("Failed to save key bindings: {}", e);
        }
        Action::Nope
    }
}

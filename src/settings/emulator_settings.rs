use std::{fs, io, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use serde::{Deserialize, Serialize};
use typetag::serde;

use crate::{
    component::{Action, Component},
    settings::{numeric_setting::NumericSetting, setting_item::SettingItem},
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub struct EmulatorSettings {
    items: Vec<Box<dyn SettingItem>>,
    #[serde(skip)]
    state: ListState,
    path: PathBuf,
}

impl EmulatorSettings {
    // Save the settings to a TOML file.
    fn save_to_file(&self, path: &PathBuf) -> io::Result<()> {
        let content = toml::to_string_pretty(&self).expect("Failed to serialize settings");
        fs::write(path, content)
    }

    // Load the settings from a TOML file.
    fn load_from_file(path: &PathBuf) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let settings: Self = toml::from_str(&content).expect("Failed to deserialize settings");
        Ok(settings)
    }

    pub fn load(path: &PathBuf) -> Self {
        if let Ok(mut data) = Self::load_from_file(path) {
            data.state = ListState::default();
            data.state.select(Some(0));
            data.path = path.clone();
            data
        } else {
            let mut data = Self::default();
            data.path = path.clone();
            data
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
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
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn decrement_current_value(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(item) = self.items.get_mut(i) {
                item.decrement();
            }
        }
    }

    fn increment_current_value(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(item) = self.items.get_mut(i) {
                item.increment();
            }
        }
    }
}

impl Default for EmulatorSettings {
    fn default() -> Self {
        let items: Vec<Box<dyn SettingItem>> = vec![
            Box::new(NumericSetting::new("Frequency", 500, 5, "Hz")),
            Box::new(NumericSetting::new("Max Deta Time", 30, 1, "ms")),
        ];

        let mut state = ListState::default();
        let path = PathBuf::from("emulator_settings.toml");
        state.select(Some(0));

        Self { items, state, path }
    }
}

impl Component for EmulatorSettings {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up => {
                self.previous();
                Action::Render
            }
            KeyCode::Down => {
                self.next();
                Action::Render
            }
            KeyCode::Left => {
                self.decrement_current_value();
                Action::Render
            }
            KeyCode::Right => {
                self.increment_current_value();
                Action::Render
            }
            _ => Action::Nope,
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| ListItem::new(item.display_value()))
            .collect();

        let widget = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Emulator Settings"),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(widget, area, &mut self.state);
    }

    fn on_exit(&mut self) -> Action {
        if let Err(e) = self.save_to_file(&self.path) {
            eprintln!("Failed to save settings: {}", e);
        }
        Action::Nope
    }
}

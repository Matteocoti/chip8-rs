use crate::{actions::Action, config_file::get_settings_file_path, constants::TITLE};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame, Terminal,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, vec};

const CHIP8_KEYS: [u8; 16] = [
    0x1, 0x2, 0x3, 0xC, 0x4, 0x5, 0x6, 0xD, 0x7, 0x8, 0x9, 0xE, 0xA, 0x0, 0xB, 0xF,
];

#[derive(Serialize, Deserialize, Clone, Debug)]
enum EmuSettingItem {
    MaxDeltaTime(u16), // MaxDeltaTime in Hz
    Frequency(u16),    // Frequency in Hz
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EmulatorSettingsData {
    items: Vec<EmuSettingItem>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct KeyboardMapData {
    mappings: HashMap<String, char>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ConfigFile {
    pub emulator: EmulatorSettingsData,
    pub keyboard: KeyboardMapData,
}

pub struct EmulatorSettings {
    items: Vec<EmuSettingItem>,
    state: ListState,
    editing_index: Option<usize>,
}

impl Default for EmulatorSettings {
    fn default() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            items: vec![
                EmuSettingItem::MaxDeltaTime(30),
                EmuSettingItem::Frequency(500),
            ],
            state,
            editing_index: None,
        }
    }
}

impl From<EmulatorSettingsData> for EmulatorSettings {
    fn from(data: EmulatorSettingsData) -> Self {
        if data.items.is_empty() {
            return Self::default();
        }
        let mut state = ListState::default();
        if !data.items.is_empty() {
            state.select(Some(0))
        }

        Self {
            items: data.items,
            state,
            editing_index: None,
        }
    }
}

impl EmulatorSettings {
    pub fn get_max_delta_time(&self) -> u16 {
        for item in self.items.iter() {
            match item {
                &EmuSettingItem::MaxDeltaTime(time) => return time,
                _ => (),
            };
        }
        panic!("EmulatorSettings must have a max delta time item");
    }

    pub fn get_frequency(&self) -> u16 {
        for item in self.items.iter() {
            match item {
                &EmuSettingItem::Frequency(freq) => return freq,
                _ => (),
            };
        }
        panic!("EmulatorSettings must have a frequency item");
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

    fn dec_value(&mut self) {
        if let Some(selected_index) = self.state.selected() {
            match self.items[selected_index] {
                EmuSettingItem::MaxDeltaTime(ref mut freq) => {
                    if *freq > 1 {
                        *freq -= 1; // Decrease frequency by 1 Hz
                    }
                }
                EmuSettingItem::Frequency(ref mut freq) => {
                    if *freq > 1 {
                        *freq -= 1; // Decrease frequency by 1 Hz
                    }
                }
            }
        }
    }

    fn inc_value(&mut self) {
        if let Some(selected_index) = self.state.selected() {
            match self.items[selected_index] {
                EmuSettingItem::MaxDeltaTime(ref mut freq) => {
                    *freq += 1; // Increase frequency by 1 Hz
                }
                EmuSettingItem::Frequency(ref mut freq) => {
                    *freq += 1; // Increase frequency by 1 Hz
                }
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
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
                self.dec_value();
                Action::Render
            }
            KeyCode::Right => {
                self.inc_value();
                Action::Render
            }
            _ => Action::Nope,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, active: bool) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let text = match item {
                    EmuSettingItem::MaxDeltaTime(delta) => {
                        format!("MaxDeltaTime: {} ms", delta)
                    }
                    EmuSettingItem::Frequency(freq) => {
                        format!("Frequency: {} Hz", freq)
                    }
                };
                ListItem::new(text)
            })
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

        if active {
            f.render_stateful_widget(widget, area, &mut self.state);
        } else {
            f.render_widget(widget, area);
        }
    }

    fn to_data(&self) -> EmulatorSettingsData {
        let mut data = EmulatorSettingsData::default();
        for item in self.items.iter() {
            match item {
                EmuSettingItem::MaxDeltaTime(freq) => {
                    data.items.push(EmuSettingItem::MaxDeltaTime(*freq))
                }
                EmuSettingItem::Frequency(freq) => {
                    data.items.push(EmuSettingItem::Frequency(*freq))
                }
            }
        }
        data
    }
}

pub struct KeyboardMap {
    keyboard: Vec<char>,
    state: ListState,
    editing_index: Option<usize>,
}

impl Default for KeyboardMap {
    fn default() -> Self {
        let keyboard = [
            '1', '2', '3', '4', 'q', 'w', 'e', 'r', 'a', 's', 'd', 'f', 'z', 'x', 'c', 'v',
        ]
        .to_vec();

        Self {
            keyboard,
            state: ListState::default(),
            editing_index: None,
        }
    }
}

impl From<KeyboardMapData> for KeyboardMap {
    fn from(data: KeyboardMapData) -> Self {
        if data.mappings.is_empty() {
            return Self::default();
        }
        let mut state = ListState::default();
        state.select(Some(0));

        let mut vec_maps: Vec<char> = vec![];

        for idx in CHIP8_KEYS {
            let string_idx = idx.to_string();
            if let Some(key) = data.mappings.get(&string_idx) {
                vec_maps.push(*key);
            } else {
                return Self::default();
            }
        }

        Self {
            keyboard: vec_maps,
            state,
            editing_index: None,
        }
    }
}

impl KeyboardMap {
    pub fn get_key_mappings(&self) -> Vec<char> {
        self.keyboard.clone()
    }

    fn previous(&mut self) {
        self.state.select(match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    Some(self.keyboard.len() - 1)
                } else {
                    Some(i - 1)
                }
            }
            None => Some(0),
        });
    }

    fn next(&mut self) {
        self.state.select(match self.state.selected() {
            Some(i) => {
                if i >= self.keyboard.len() - 1 {
                    Some(0)
                } else {
                    Some(i + 1)
                }
            }
            None => Some(0),
        });
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
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

    pub fn render(&mut self, frame: &mut Frame, area: Rect, active: bool) {
        let items: Vec<ListItem> = self
            .keyboard
            .iter()
            .enumerate()
            .map(|(chip8_key, qwerty_key)| {
                // Formatta ogni riga per mostrare la mappatura
                let line = format!("0x{:X}  -> '{}'", chip8_key, qwerty_key);
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

        if self.editing_index.is_some() {
            widget = widget.highlight_style(Style::default().fg(Color::Black).bg(Color::White));
        }
        if active {
            frame.render_stateful_widget(widget, area, &mut self.state);
        } else {
            frame.render_widget(widget, area);
        }
    }

    fn to_data(&self) -> KeyboardMapData {
        let mut key_maps = HashMap::new();

        for (idx, qwerty_key) in self.keyboard.iter().enumerate() {
            key_maps.insert(CHIP8_KEYS[idx].to_string(), *qwerty_key);
        }

        KeyboardMapData { mappings: key_maps }
    }
}

pub struct Settings {
    active_column: u8,
    number_of_columns: u8,
    emu_settings: EmulatorSettings,
    keyboard: KeyboardMap,
}

impl Settings {
    pub fn new() -> Self {
        let config_data: ConfigFile = get_settings_file_path()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default();

        let emu_settings = EmulatorSettings::from(config_data.emulator);
        let keyboard = KeyboardMap::from(config_data.keyboard);

        Self {
            active_column: 0,
            emu_settings,
            keyboard,
            number_of_columns: 2,
        }
    }

    fn next_column(&mut self) {
        self.active_column = (self.active_column + 1) % self.number_of_columns;
    }

    pub fn get_max_delta_time(&self) -> u16 {
        self.emu_settings.get_max_delta_time()
    }

    pub fn get_frequency(&self) -> u16 {
        self.emu_settings.get_frequency()
    }

    pub fn get_key_mappings(&self) -> Vec<char> {
        self.keyboard.get_key_mappings()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Tab => {
                self.next_column();
                Action::Render
            }
            KeyCode::Esc => {
                let _ = self.save();
                Action::GoToMenu
            }
            _ => {
                if self.active_column == 0 {
                    self.emu_settings.handle_key_event(key)
                } else {
                    self.keyboard.handle_key_event(key)
                }
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Ratio(1, 5),
                    Constraint::Ratio(3, 5),
                    Constraint::Ratio(1, 5),
                ]
                .as_ref(),
            )
            .split(f.area());
        let title_area = main_chunks[0];
        let content_area = main_chunks[1];
        let footer_area = main_chunks[2];

        // Split the content area into two horizontal chunks
        // for the emulator settings and keyboard map
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
            .split(content_area);

        // Title section
        let title_paragraph = Paragraph::new(TITLE)
            .alignment(Alignment::Center)
            .block(Block::default());
        f.render_widget(title_paragraph, title_area);

        // Render of the two settings sections inside the content area
        self.emu_settings
            .render(f, horizontal_chunks[0], self.active_column == 0);
        self.keyboard
            .render(f, horizontal_chunks[1], self.active_column == 1);

        // Footer section
        let help_line: Line = if self.keyboard.editing_index.is_some() {
            Line::from("Press a key to map | [ESC] to cancel ").alignment(Alignment::Center)
        } else {
            let mut line = match self.active_column {
                0 => Line::from("[←→] Change value").alignment(Alignment::Center),
                1 => Line::from("[↑↓] Navigate | [Enter] Edit").alignment(Alignment::Center),
                _ => Line::default(),
            };

            let common_line = Line::from(" | [TAB] Switch panel | [ESC] Save & exit")
                .alignment(Alignment::Center);

            line.extend(common_line);

            line
        };

        let help_paragraph = Paragraph::new(help_line);
        f.render_widget(help_paragraph, footer_area);
    }

    pub fn save(&self) -> Result<()> {
        let emu_data = self.emu_settings.to_data();
        let key_data = self.keyboard.to_data();

        let content = ConfigFile {
            emulator: emu_data,
            keyboard: key_data,
        };

        let config_path = get_settings_file_path().expect("Impossible to save settings data");

        if let Some(parent_dir) = config_path.parent() {
            let _ = fs::create_dir_all(parent_dir);
        }

        let toml_string = toml::to_string_pretty(&content)?;

        fs::write(config_path, toml_string)?;

        Ok(())
    }
}

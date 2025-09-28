use crate::actions::Action;
use crate::config_file::get_rom_path;
use crate::constants::TITLE;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Paragraph, Row, Table, TableState,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
struct RomFileData {
    path: String,
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct SavedRoms {
    roms: Vec<RomFileData>,
}

pub struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

pub struct RomBrowser {
    path: PathBuf,
    items: Vec<Entry>,
    state: TableState,
    show_hidden_files: bool,
    filter: String,
    editing: bool,
}

pub struct RomHistory {
    roms: Vec<RomFileData>,
    roms_state: ListState,
}

pub struct RomFinder {
    browser: RomBrowser,
    history: RomHistory,
    active_column: u8,
    number_of_colums: u8,
}

fn is_path_hidden(path: &Path) -> io::Result<bool> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        let metadata = fs::metadata(path)?;
        let attributes = metadata.file_attributes();
        const HIDDEN_FLAG: u32 = 0x2;
        Ok((attributes & HIDDEN_FLAG) != 0)
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(file_name_osstr) = path.file_name() {
            if let Some(file_name) = file_name_osstr.to_str() {
                return Ok(file_name.starts_with('.'));
            }
        }
        Ok(false)
    }
}

/// Helper method to verify if a given directory
/// is associated with at least one entry
fn dir_has_entry(path: &Path) -> io::Result<bool> {
    let mut entries = fs::read_dir(path)?;

    Ok(entries.next().is_some())
}

impl RomBrowser {
    pub fn new() -> Self {
        // The finder starting path would be the home directory of the file
        // system
        let mut path = PathBuf::from(".");

        if let Some(home_dir) = home::home_dir() {
            path = home_dir;
        }

        let mut browser = Self {
            path,
            items: vec![],
            state: TableState::default(),
            show_hidden_files: false,
            filter: String::new(),
            editing: false,
        };

        let _ = browser.update_data();

        browser
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

    fn update_data(&mut self) -> io::Result<()> {
        self.items.clear();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let path = entry.path();
            if let Ok(path_hidden) = is_path_hidden(&path) {
                if !self.show_hidden_files && path_hidden {
                    continue;
                }
            }
            if let Some(os_name) = path.file_name() {
                let name = os_name.to_string_lossy().into_owned();

                if !self.editing || !self.filter.is_empty() && name.starts_with(&self.filter) {
                    let is_dir = path.is_dir();
                    if is_dir {
                        if let Ok(entries) = dir_has_entry(&path) {
                            if entries {
                                self.items.push(Entry { name, path, is_dir });
                            }
                        }
                    } else {
                        self.items.push(Entry { name, path, is_dir })
                    }
                }
            }
        }
        self.state.select(Some(0));
        Ok(())
    }

    fn handle_enter_key(&mut self) -> Action {
        if let Some(selected_index) = self.state.selected() {
            if let Some(entry) = self.items.get(selected_index) {
                if entry.is_dir {
                    self.path = self.path.join(&entry.name);
                    self.filter.clear();
                    self.editing = false;
                    let _ = self.update_data();
                    Action::Render
                } else {
                    Action::LoadRom(self.path.join(entry.name.clone()))
                }
            } else {
                Action::Nope
            }
        } else {
            Action::Nope
        }
    }

    /// Handle the left key event.
    ///
    /// Tries to truncate the path to the parent folder.
    /// If it does, the data are updated and the render action
    /// is returned.
    fn handle_back_key(&mut self) -> Action {
        if self.path.pop() {
            let _ = self.update_data();
            Action::Render
        } else {
            Action::Nope
        }
    }

    pub fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
        if self.editing {
            match evt.code {
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    let _ = self.update_data();
                    Action::Render
                }
                KeyCode::Esc => {
                    self.filter.clear();
                    self.editing = false;
                    let _ = self.update_data();
                    Action::Render
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                    Action::Render
                }
                KeyCode::Enter | KeyCode::Right => self.handle_enter_key(),
                _ => Action::Nope,
            }
        } else {
            match evt.code {
                KeyCode::Char('w') | KeyCode::Char('k') | KeyCode::Up => {
                    self.previous();
                    Action::Render
                }
                KeyCode::Char('s') | KeyCode::Char('j') | KeyCode::Down => {
                    self.next();
                    Action::Render
                }
                KeyCode::Enter | KeyCode::Right => self.handle_enter_key(),
                KeyCode::Left => self.handle_back_key(),
                KeyCode::Esc => Action::GoToMenu,
                KeyCode::Char('/') => {
                    self.editing = true;
                    Action::Render
                }
                KeyCode::Char('h') => {
                    if evt.modifiers.contains(KeyModifiers::CONTROL) {
                        self.show_hidden_files = !self.show_hidden_files;
                        let _ = self.update_data();
                        Action::Render
                    } else {
                        Action::Nope
                    }
                }
                _ => Action::Nope,
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, active: bool) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
            .split(area);

        let browser_area = chunks[0];
        let filter_area = chunks[1];

        let rows = self.items.iter().map(|entry| {
            let (icon, style) = if entry.is_dir {
                ("📁", Style::default().fg(Color::Blue))
            } else {
                ("📄", Style::default().fg(Color::White))
            };
            Row::new(vec![icon, &entry.name]).style(style)
        });

        let widths = [Constraint::Length(3), Constraint::Min(0)];
        let table = Table::new(rows, widths)
            .block(Block::default().borders(Borders::ALL).title("ROM Browser"))
            .highlight_symbol(">> ");

        if self.editing {
            f.render_stateful_widget(table, browser_area, &mut self.state);

            let filter_paragraph = Paragraph::new(format!("Filter: {}", self.filter))
                .alignment(Alignment::Left)
                .block(Block::default());

            f.render_widget(filter_paragraph, filter_area);
        } else if active {
            f.render_stateful_widget(table, area, &mut self.state);
        } else {
            f.render_widget(table, area);
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn render_footer(&self) -> Line {
        // Definisci gli stili come prima
        let key_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let text_style = Style::default().fg(Color::Gray);

        if self.editing {
            Line::from(vec![
                Span::styled("[ESC]", key_style),
                Span::styled(" to cancel", text_style),
                Span::raw(" | "),
                Span::styled("[Enter]", key_style),
                Span::styled(" to select", text_style),
            ])
        } else {
            Line::from(vec![
                Span::styled("[↑↓]", key_style),
                Span::styled(" Nav", text_style),
                Span::raw(" | "),
                Span::styled("[←] ", key_style),
                Span::styled("Back", text_style),
                Span::raw(" | "),
                Span::styled("[→/Enter]", key_style),
                Span::styled(" Open", text_style),
                Span::raw(" | "),
                Span::styled("[/]", key_style),
                Span::styled(" Filter", text_style),
            ])
        }
    }
}

impl RomHistory {
    pub fn new() -> Self {
        let roms = Self::load_roms().unwrap_or_default();
        let mut roms_state = ListState::default();

        if !roms.is_empty() {
            roms_state.select(Some(0));
        }

        Self { roms, roms_state }
    }

    fn next(&mut self) {
        let i = match self.roms_state.selected() {
            Some(i) => {
                if i >= self.roms.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.roms_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.roms_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.roms.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.roms_state.select(Some(i));
    }

    fn load_roms() -> io::Result<Vec<RomFileData>> {
        let config_path = get_rom_path().unwrap();

        if !config_path.exists() {
            return Ok(Vec::new());
        }

        let toml_content = fs::read_to_string(config_path)?;
        let saved_data: SavedRoms = toml::from_str(&toml_content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(saved_data.roms)
    }

    pub fn register_rom(&mut self, rom_path: PathBuf) {
        let name = rom_path.file_name().unwrap().to_string_lossy().into_owned();
        let path = rom_path.parent().unwrap().to_string_lossy().into_owned();

        let rom = RomFileData { path, name };

        if !self.roms.contains(&rom) {
            self.roms.push(rom);
        }
    }

    pub fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
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
                if let Some(selected_index) = self.roms_state.selected() {
                    if selected_index < self.roms.len() {
                        let rom = &self.roms[selected_index];
                        return Action::LoadRom(PathBuf::from(&rom.path).join(&rom.name));
                    }
                }
                Action::Nope
            }
            _ => Action::Nope,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, active: bool) {
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

        if active {
            f.render_stateful_widget(roms_list, area, &mut self.roms_state);
        } else {
            f.render_widget(roms_list, area);
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

    pub fn save(&self) -> io::Result<()> {
        let config_path = get_rom_path().unwrap();

        if let Some(dir) = config_path.parent() {
            fs::create_dir_all(dir)?;
        }

        let saved_data = SavedRoms {
            roms: self.roms.clone(),
        };
        let toml_content =
            toml::to_string(&saved_data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        fs::write(config_path, toml_content)
    }
}

impl RomFinder {
    pub fn new() -> Self {
        Self {
            browser: RomBrowser::new(),
            history: RomHistory::new(),
            active_column: 0,
            number_of_colums: 2,
        }
    }

    fn next_column(&mut self) {
        self.active_column = (self.active_column + 1) % self.number_of_colums;
    }

    pub fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
        let action = match self.active_column {
            0 => self.browser.handle_key_event(evt),
            1 => self.history.handle_key_event(evt),
            _ => Action::Nope,
        };

        if let Action::Nope = action {
            match evt.code {
                KeyCode::Tab => {
                    self.next_column();
                    Action::Render
                }
                KeyCode::Esc => Action::GoToMenu,
                _ => Action::Nope,
            }
        } else {
            action
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 5), Constraint::Ratio(4, 5)].as_ref())
            .split(f.area());

        // Title
        let title_paragraph = Paragraph::new(TITLE)
            .alignment(Alignment::Center)
            .block(Block::default());
        f.render_widget(title_paragraph, vertical_chunks[0]);

        let browser_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(vertical_chunks[1]);

        let header = browser_chunks[0];
        let content_area = browser_chunks[1];
        let footer_area = browser_chunks[2];

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)].as_ref())
            .split(content_area);

        let browser_chunk = main_chunks[0];
        let history = main_chunks[1];

        self.browser
            .render(f, browser_chunk, self.active_column == 0);
        self.history.render(f, history, self.active_column == 1);

        let mut footer = match self.active_column {
            0 => self.browser.render_footer().spans,
            1 => self.history.render_footer().spans,
            _ => vec![],
        };

        if !footer.is_empty() {
            let key_style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD);
            let text_style = Style::default().fg(Color::Gray);
            footer.push(Span::styled(" | ", text_style));
            footer.push(Span::styled("[TAB]", key_style));
            footer.push(Span::styled(" Switch panel", text_style));

            if !self.browser.is_editing() {
                footer.push(Span::styled(" | ", text_style));
                footer.push(Span::styled("[ESC]", key_style));
                footer.push(Span::styled(" Go to main menu", text_style));
            }
        }

        let help_line = Line::from(footer).alignment(Alignment::Right);
        f.render_widget(help_line, footer_area);
    }

    pub fn register_rom(&mut self, rom_path: PathBuf) {
        self.history.register_rom(rom_path);
    }

    pub fn save(&self) -> io::Result<()> {
        self.history.save()
    }
}

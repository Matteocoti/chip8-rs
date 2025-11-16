use std::{
    fs, io,
    path::{Path, PathBuf},
};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};

use crate::{
    chip8_tui::Chip8TUI,
    component::{Action, Component, Transition},
};

pub struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

pub struct FileBrowser {
    path: PathBuf,
    items: Vec<Entry>,
    state: TableState,
    show_hidden_files: bool,
    filter: String,
    editing: bool,
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

impl FileBrowser {
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
                    Action::Transition(Transition::Switch(Box::new(Chip8TUI::new(&entry.path))))
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

    #[allow(dead_code)]
    pub fn is_editing(&self) -> bool {
        self.editing
    }

    #[allow(dead_code)]
    pub fn render_footer(&self) -> Line {
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

impl Component for FileBrowser {
    fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
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
                KeyCode::Esc => Action::Transition(Transition::Pop),
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

    fn render(&mut self, f: &mut Frame, area: Rect) {
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
        } else {
            f.render_stateful_widget(table, area, &mut self.state);
        }
    }
}

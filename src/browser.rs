use crate::actions::Action;
use crate::constants::TITLE;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, TableState};
use std::fs;
use std::io;
use std::path::PathBuf;

pub struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

pub struct RomFinder {
    path: PathBuf,
    items: Vec<Entry>,
    state: TableState,
}

impl RomFinder {
    pub fn new() -> Self {
        // The finder starting path would be the home directory of the file
        // system
        let mut path = PathBuf::from(".");

        if let Some(home_dir) = home::home_dir() {
            path = home_dir;
        }

        let mut finder = Self {
            path,
            items: vec![],
            state: TableState::default(),
        };

        let _ = finder.update_data();

        finder
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
            if let Some(os_name) = path.file_name() {
                let name = os_name.to_string_lossy().into_owned();
                let is_dir = path.is_dir();
                self.items.push(Entry { name, path, is_dir })
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
                    let _ = self.update_data();
                } else {
                    return Action::LoadRom(self.path.join(entry.name.clone()));
                }
            }
        }
        Action::Nope
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
            _ => Action::Nope,
        }
    }

    pub fn render(&mut self, terminal: &mut ratatui::Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _ = terminal.draw(|f| {
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

            f.render_stateful_widget(table, browser_chunks[1], &mut self.state);
        });
    }
}

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::Alignment,
    style::Style,
    widgets::{Block, ListState, Paragraph},
};

use crate::actions::Action;
use crate::constants::{SUB_TITLE, TITLE};

pub struct MainMenu {
    items: Vec<String>,
    state: ListState,
}

impl MainMenu {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            items: vec![
                "Start Game".to_string(),
                "Load Rom".to_string(),
                "Options".to_string(),
                "Quit".to_string(),
            ],
            state,
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

    pub fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
        let code = evt.code;
        match code {
            KeyCode::Char('w') | KeyCode::Char('k') | KeyCode::Up => {
                self.previous();
                Action::Render
            }
            KeyCode::Char('s') | KeyCode::Char('j') | KeyCode::Down => {
                self.next();
                Action::Render
            }
            KeyCode::Enter => match self.state.selected().unwrap() {
                0 => Action::GoToGame,
                1 => Action::GoToRomFinder,
                2 => Action::GoToSetting,
                3 => Action::Quit,
                _ => Action::Nope,
            },
            _ => Action::Nope,
        }
    }

    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _ = terminal.draw(|f| {
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Ratio(1, 5),
                        Constraint::Ratio(1, 5),
                        Constraint::Ratio(2, 5),
                        Constraint::Ratio(1, 5),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Title
            let title_paragraph = Paragraph::new(TITLE)
                .alignment(Alignment::Center)
                .block(Block::default());
            f.render_widget(title_paragraph, vertical_chunks[0]);

            // Subtitle
            let subtitle_paragraph = Paragraph::new(SUB_TITLE)
                .alignment(Alignment::Center)
                .block(Block::default());
            f.render_widget(subtitle_paragraph, vertical_chunks[1]);

            let selected_index = self.state.selected();
            let menu_lines: Vec<Line> = self
                .items
                .iter()
                .enumerate() // Usiamo enumerate() per ottenere l'indice di ogni elemento
                .map(|(index, item_text)| {
                    if Some(index) == selected_index {
                        let styled_text = format!(">> {} <<", item_text);
                        Line::from(Span::styled(
                            styled_text,
                            Style::default().add_modifier(Modifier::BOLD),
                        ))
                    } else {
                        Line::from(item_text.clone())
                    }
                })
                .collect();

            let menu_paragraph = Paragraph::new(menu_lines)
                .alignment(Alignment::Center)
                .block(Block::default());

            f.render_widget(menu_paragraph, vertical_chunks[2]);
        });
    }
}

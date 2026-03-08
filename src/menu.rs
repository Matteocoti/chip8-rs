use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::Alignment,
    style::Style,
    widgets::{Block, ListState, Paragraph},
};

use crate::component::{Action, Component, Transition};
use crate::config_manager::ConfigManager;
use crate::constants::{SUB_TITLE, TITLE};
use crate::file_browser::FileBrowser;
use crate::rom_history::RomHistory;
use crate::settings::{EmulatorSettings, KeyBindings};
use crate::split_view_component::SplitViewComponent;

/// Represents the main menu screen of the application.
///
/// This component manages the state for navigating and selecting options
/// from the main menu, such as loading a ROM, changing settings, or quitting.
pub struct MainMenu {
    /// The list of menu item labels.
    items: Vec<String>,
    /// The state that tracks the currently selected item.
    state: ListState,
    /// Configuration manager for application settings.
    config: ConfigManager,
}

/// Inherent methods for the MainMenu.
impl MainMenu {
    /// Creates a new `MainMenu` instance.
    ///
    /// Initializes the menu with a predefined list of items and sets the
    /// selection to the first item by default.
    pub fn new(config: ConfigManager) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            items: vec![
                "Load Rom".to_string(),
                "Options".to_string(),
                "Quit".to_string(),
            ],
            state,
            config,
        }
    }

    /// Moves the selection to the next item in the menu.
    ///
    /// If the last item is selected, it wraps around to the first item.
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

    /// Moves the selection to the previous item in the menu.
    ///
    /// If the first item is selected, it wraps around to the last item.
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
}

/// Implementation of the `Component` trait for the MainMenu.
///
/// This handles the user input and rendering logic for the main menu screen.
impl Component for MainMenu {
    /// Handles key press events for the main menu.
    ///
    /// - `Up` (`w`, `k`): Navigates to the previous menu item.
    /// - `Down` (`s`, `j`): Navigates to the next menu item.
    /// - `Enter`: Selects the current item and returns an `Action` to transition
    ///   to the corresponding screen (`RomFinder`, `Settings`) or to quit the application.
    fn handle_key_event(&mut self, evt: KeyEvent) -> Action {
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
                0 => {
                    let new_component = SplitViewComponent::builder()
                        .pane(Box::new(FileBrowser::new(self.config.clone())))
                        .pane(Box::new(RomHistory::load(
                            &self.config.rom_history_path,
                            self.config.clone(),
                        )))
                        .direction(Direction::Horizontal)
                        .build();

                    match new_component {
                        Ok(comp) => Action::Transition(Transition::Push(Box::new(comp))),
                        Err(_) => Action::Nope,
                    }
                }
                1 => {
                    let new_component = SplitViewComponent::builder()
                        .pane(Box::new(EmulatorSettings::load(
                            &self.config.emulator_settings_path,
                        )))
                        .pane(Box::new(KeyBindings::load(&self.config.key_bindings_path)))
                        .direction(Direction::Horizontal)
                        .build();

                    match new_component {
                        Ok(comp) => Action::Transition(Transition::Push(Box::new(comp))),
                        Err(_) => Action::Nope,
                    }
                }
                2 => Action::Quit,
                _ => Action::Nope,
            },
            _ => Action::Nope,
        }
    }

    /// Renders the main menu user interface.
    ///
    /// This method draws the application title, subtitle, and a centered list
    /// of menu items to the frame. The currently selected item is highlighted.
    fn render(&mut self, f: &mut Frame, area: Rect) {
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
            .split(area);

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
    }
}

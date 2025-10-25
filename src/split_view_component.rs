use crate::component::{Action, Component, Transition};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

/// An error that can occur when building a `SplitViewComponent`.
#[derive(Debug)]
pub enum BuildError {
    /// The builder was missing a direction for the layout.
    MissingDirection,
    /// The builder was not given any panes to display.
    MissingPanes,
}

/// A generic container that displays multiple components in a split view.
///
/// This component manages the layout and focus for a collection of child
/// components (panes), arranging them either horizontally or vertically.
pub struct SplitViewComponent {
    /// Child components
    panes: Vec<Box<dyn Component>>,
    /// The idx of the current active plane
    active_idx: usize,
    /// Direction to split the layout
    direction: Direction,
}

impl SplitViewComponent {
    ///  Returns a new builder for creating a 'SplitViewComponent'
    pub fn builder() -> SplitViewComponentBuilder {
        SplitViewComponentBuilder::new()
    }
}

impl Component for SplitViewComponent {
    /// Handles key events for the container.
    ///
    /// - `Tab`: Switches focus to the next pane in the view.
    /// - Other keys: Are delegated to the currently active child pane.
    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Action {
        // The tab key changes the focus of the compoment
        if key.code == KeyCode::Tab {
            self.active_idx = (self.active_idx + 1) % self.panes.len();
            return Action::Render;
        }

        if key.code == KeyCode::Esc {
            return Action::Transition(Transition::Pop);
        }
        // The other events are delegate to the active pane
        if let Some(active_pane) = self.panes.get_mut(self.active_idx) {
            return active_pane.handle_key_event(key);
        }

        Action::Nope
    }

    /// Renders the split view and all its child panes.
    fn render(&mut self, f: &mut Frame, area: Rect) {
        if self.panes.is_empty() {
            return;
        }
        // Creates a layout based on the direction and the number of panes
        let constraints = std::iter::repeat(Constraint::Fill(1))
            .take(self.panes.len())
            .collect::<Vec<_>>();
        let layout = Layout::default()
            .direction(self.direction)
            .constraints(constraints)
            .split(area);
        // Render each pane
        for (i, pane) in self.panes.iter_mut().enumerate() {
            pane.render(f, layout[i])
        }
    }

    fn on_entry(&mut self) -> Action {
        for pane in self.panes.iter_mut() {
            pane.on_entry();
        }

        Action::Nope
    }

    fn on_exit(&mut self) -> Action {
        for pane in self.panes.iter_mut() {
            pane.on_exit();
        }

        Action::Nope
    }
}

// --- Builder Implementation ---

/// A builder for creating `SplitViewComponent` instances.
pub struct SplitViewComponentBuilder {
    panes: Vec<Box<dyn Component>>,
    direction: Option<Direction>,
}

impl SplitViewComponentBuilder {
    /// Creates a new, empty builder.
    fn new() -> Self {
        Self {
            panes: Vec::new(),
            direction: None,
        }
    }

    /// Adds a component pane to the split view.
    pub fn pane(mut self, pane: Box<dyn Component>) -> Self {
        self.panes.push(pane);
        self
    }

    /// Sets the direction of the split (Horizontal or Vertical).
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Consumes the builder and creates the SplitViewComponent.
    ///
    /// This method will return an error if required fields are missing.
    pub fn build(self) -> Result<SplitViewComponent, BuildError> {
        if self.panes.is_empty() {
            return Err(BuildError::MissingPanes);
        }

        let direction = self.direction.ok_or(BuildError::MissingDirection)?;

        Ok(SplitViewComponent {
            panes: self.panes,
            active_idx: 0,
            direction,
        })
    }
}

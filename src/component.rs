use ratatui::crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

/// Component trait that each TUI object has to implement to be rendered to the screen
pub trait Component {
    /// Handle key press/repeat events and return an action for the application handler
    fn handle_key_event(&mut self, key: KeyEvent) -> Action;

    /// Handle key release events. Only needs to be overridden by components that
    /// track held-key state (e.g. the emulator). Default is a no-op.
    fn handle_key_release(&mut self, _key: KeyEvent) -> Action {
        Action::Nope
    }

    /// Render the component's UI to the frame
    fn render(&mut self, f: &mut Frame, area: Rect);

    /// Update the component's state
    fn update(&mut self) -> Action {
        Action::Nope
    }

    fn on_entry(&mut self) -> Action {
        Action::Nope
    }

    fn on_exit(&mut self) -> Action {
        Action::Nope
    }
}

pub enum Transition {
    /// Pop a component from the stack
    Pop,
    /// Push a new component to the stack
    Push(Box<dyn Component>),
    /// Switch to a new component
    Switch(Box<dyn Component>),
}

/// Enumerator to define the type of action that the application has
/// to execute
pub enum Action {
    /// Do Nothing
    Nope,
    /// Render the application
    Render,
    /// Transition from one component to another
    Transition(Transition),
    /// Quit the application
    Quit,
}

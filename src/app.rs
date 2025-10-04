use crate::component::{self, Action, Component};
use crate::config_manager::ConfigManager;
use crate::menu::MainMenu;
use crate::performance_metrics::PerformanceMetrics;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::layout::{Constraint, Direction, Layout};
use std::fs::{File, OpenOptions};
use std::io::stdout;
use std::time::Duration;

pub struct App {
    should_quit: bool,
    stack: Vec<Box<dyn Component>>,
    lofg: File,                  // Optional log file path
    metrics: PerformanceMetrics, // Performance metrics tracker
    config: ConfigManager,
}

fn init_tui_terminal() -> color_eyre::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}
fn restore_terminal() -> color_eyre::Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

impl App {
    pub fn new() -> Self {
        let mut options = OpenOptions::new();
        let logf = options.append(true).create(true).open("foo.txt").unwrap();

        let config = ConfigManager::new();

        let main_menu = Box::new(MainMenu::new(config.clone()));

        Self {
            should_quit: false,
            stack: vec![main_menu],
            lofg: logf,
            metrics: PerformanceMetrics::new(200),
            config,
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        let mut needs_render = false;

        match action {
            Action::Quit => self.should_quit = true,
            Action::Render => needs_render = true,
            Action::Transition(transition) => match transition {
                crate::component::Transition::None => (),
                crate::component::Transition::Pop => {
                    let component = self.stack.pop();
                    if let Some(mut pane) = component {
                        pane.on_exit();
                    }
                    needs_render = true;
                }
                crate::component::Transition::Push(mut component) => {
                    component.on_entry();
                    self.stack.push(component);
                    needs_render = true;
                }
                crate::component::Transition::Switch(mut component) => {
                    if !self.stack.is_empty() {
                        let component = self.stack.pop();
                        if let Some(mut pane) = component {
                            pane.on_exit();
                        }
                    }
                    component.on_entry();
                    self.stack.push(component);
                    needs_render = true;
                }
            },
            _ => (),
        }

        needs_render
    }

    pub fn run(&mut self) -> Result<(), color_eyre::Report> {
        let mut terminal = init_tui_terminal()?;

        let target_frame_duration = Duration::from_secs_f64(1.0 / 60.0);

        self.render(&mut terminal);

        'main_loop: loop {
            let frame_start = self.metrics.start_frame();
            let mut needs_redraw = false;
            while crossterm::event::poll(Duration::ZERO)? {
                if let Ok(Event::Key(key_event)) = crossterm::event::read() {
                    match key_event.code {
                        KeyCode::Char('c') => {
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                self.should_quit = true;
                                break;
                            }
                        }
                        KeyCode::F(12) => self.metrics.toggle_visibility(),
                        _ => (),
                    }

                    let action = self.handle_events(key_event);
                    needs_redraw |= self.handle_action(action);
                }
            }

            if self.should_quit {
                break 'main_loop;
            }
            let update_action = self.update();
            needs_redraw |= self.handle_action(update_action);
            if needs_redraw {
                self.render(&mut terminal);
            }
            let elapsed = self.metrics.end_frame(frame_start);
            if elapsed < target_frame_duration {
                std::thread::sleep(target_frame_duration - elapsed);
            }
        }

        Ok(())
    }

    fn update(&mut self) -> Action {
        let component = self.stack.last_mut().unwrap();
        component.update()
    }

    fn handle_events(&mut self, event: KeyEvent) -> Action {
        let component = self.stack.last_mut().unwrap();
        component.handle_key_event(event)
    }

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _ = terminal.draw(|f| {
            let component = self.stack.last_mut().unwrap();
            let area = f.area();
            component.render(f, area);

            // After the mode has rendered, overlay the performance metrics if visible
            if self.metrics.is_visible() {
                // Create a small area at the bottom of the screen for metrics
                let metrics_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(0),
                            Constraint::Length(1), // Just a single line
                        ]
                        .as_ref(),
                    )
                    .split(f.area())[1];
                self.metrics.render(f, metrics_area);
            }
        });
    }
}

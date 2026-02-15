use crate::component::{Action, Component};
use crate::config_manager::ConfigManager;
use crate::menu::MainMenu;
use crate::rom_history::RomHistory;
use crate::performance_metrics::PerformanceMetrics;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    ExecutableCommand,
    event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags, poll, read},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement},
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Clear, Paragraph};
use std::fs::{File, OpenOptions};
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

const NOTIFICATION_DURATION: Duration = Duration::from_secs(5);

pub struct App {
    should_quit: bool,
    stack: Vec<Box<dyn Component>>,
    log: File,
    metrics: PerformanceMetrics,
    #[allow(dead_code)]
    config: ConfigManager,
    notification: Option<(String, Instant)>,
}

fn init_tui_terminal() -> color_eyre::Result<(Terminal<CrosstermBackend<std::io::Stdout>>, bool)> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let keyboard_enhanced = supports_keyboard_enhancement().unwrap_or(false);
    if keyboard_enhanced {
        stdout().execute(PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
        ))?;
    }
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok((terminal, keyboard_enhanced))
}
fn restore_terminal(keyboard_enhanced: bool) -> color_eyre::Result<()> {
    if keyboard_enhanced {
        stdout().execute(PopKeyboardEnhancementFlags)?;
    }
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

impl App {
    pub fn new() -> Self {
        let config = ConfigManager::new();

        let mut options = OpenOptions::new();
        let logf = options
            .append(true)
            .create(true)
            .open(&config.log_path)
            .unwrap();

        let main_menu = Box::new(MainMenu::new(config.clone()));

        Self {
            should_quit: false,
            stack: vec![main_menu],
            log: logf,
            metrics: PerformanceMetrics::new(200),
            config,
            notification: None,
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        let mut needs_render = false;

        match action {
            Action::Quit => self.should_quit = true,
            Action::Render => needs_render = true,
            Action::RegisterRom(path) => {
                let mut history = RomHistory::load(&self.config.rom_history_path);
                history.register_rom(path);
                let _ = history.save_to_file(&self.config.rom_history_path);
            }
            Action::Notify(msg) => {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(self.log, "[{timestamp}] {msg}");
                self.notification = Some((msg, Instant::now()));
                needs_render = true;
            }
            Action::Transition(transition) => match transition {
                crate::component::Transition::Pop => {
                    let component = self.stack.pop();
                    if let Some(mut pane) = component {
                        let action = pane.on_exit();
                        self.handle_action(action);
                    }
                    if self.stack.is_empty() {
                        self.should_quit = true;
                    }
                    needs_render = true;
                }
                crate::component::Transition::Push(mut component) => {
                    let action = component.on_entry();
                    self.stack.push(component);
                    self.handle_action(action);
                    needs_render = true;
                }
                crate::component::Transition::Switch(mut component) => {
                    if !self.stack.is_empty() {
                        let component = self.stack.pop();
                        if let Some(mut pane) = component {
                            let action = pane.on_exit();
                            self.handle_action(action);
                        }
                    }
                    let action = component.on_entry();
                    self.stack.push(component);
                    self.handle_action(action);
                    needs_render = true;
                }
            },
            _ => (),
        }

        needs_render
    }

    pub fn run(&mut self) -> Result<(), color_eyre::Report> {
        let (mut terminal, keyboard_enhanced) = init_tui_terminal()?;

        let target_frame_duration = Duration::from_secs_f64(1.0 / 60.0);

        self.render(&mut terminal);

        'main_loop: loop {
            let frame_start = self.metrics.start_frame();
            while poll(Duration::ZERO)? {
                if let Ok(Event::Key(key_event)) = read() {
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

                    let action = if key_event.kind == KeyEventKind::Release {
                        self.handle_key_release(key_event)
                    } else {
                        self.handle_events(key_event)
                    };
                    self.handle_action(action);
                }
            }

            if self.should_quit {
                break 'main_loop;
            }

            // Expire notification after 5 seconds
            if let Some((_, when)) = &self.notification {
                if when.elapsed() >= NOTIFICATION_DURATION {
                    self.notification = None;
                }
            }

            let update_action = self.update();
            self.handle_action(update_action);
            self.render(&mut terminal);
            let elapsed = self.metrics.end_frame(frame_start);
            if elapsed < target_frame_duration {
                std::thread::sleep(target_frame_duration - elapsed);
            }
        }

        restore_terminal(keyboard_enhanced)?;
        Ok(())
    }

    fn update(&mut self) -> Action {
        match self.stack.last_mut() {
            Some(component) => component.update(),
            None => Action::Nope,
        }
    }

    fn handle_events(&mut self, event: KeyEvent) -> Action {
        match self.stack.last_mut() {
            Some(component) => component.handle_key_event(event),
            None => Action::Nope,
        }
    }

    fn handle_key_release(&mut self, event: KeyEvent) -> Action {
        match self.stack.last_mut() {
            Some(component) => component.handle_key_release(event),
            None => Action::Nope,
        }
    }

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _ = terminal.draw(|f| {
            let area = f.area();
            if let Some(component) = self.stack.last_mut() {
                component.render(f, area);
            }

            // Overlay the performance metrics if visible
            if self.metrics.is_visible() {
                let metrics_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            Constraint::Min(0),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.area())[1];
                self.metrics.render(f, metrics_area);
            }

            // Overlay the notification if active and not expired
            if let Some((ref msg, when)) = self.notification {
                if when.elapsed() < NOTIFICATION_DURATION {
                    let notif_area = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(
                            [
                                Constraint::Min(0),
                                Constraint::Length(1),
                            ]
                            .as_ref(),
                        )
                        .split(area)[1];
                    f.render_widget(Clear, notif_area);
                    f.render_widget(
                        Paragraph::new(msg.as_str())
                            .style(Style::default().bg(Color::Yellow).fg(Color::Black)),
                        notif_area,
                    );
                }
            }
        });
    }
}

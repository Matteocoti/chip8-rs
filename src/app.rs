use crate::actions::Action;
use crate::browser::RomFinder;
use crate::chip8_tui::Chip8TUI;
use crate::menu::MainMenu;
use crate::performance_metrics::PerformanceMetrics;
use crate::settings::Settings;
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

pub enum Mode {
    Menu,
    RomSelection,
    Settings,
    Game,
}

pub struct App {
    mode: Mode, // Application state
    should_quit: bool,
    settings: Settings,          // Application settings
    menu: MainMenu,              // Main menu component
    finder: RomFinder,           // Rom finder component
    emu: Chip8TUI,               // Emulator component
    lofg: File,                  // Optional log file path
    metrics: PerformanceMetrics, // Performance metrics tracker
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

        Self {
            mode: Mode::Menu,
            should_quit: false,
            settings: Settings::new(),
            menu: MainMenu::new(),
            finder: RomFinder::new(),
            emu: Chip8TUI::new(),
            lofg: logf,
            metrics: PerformanceMetrics::new(200),
        }
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

                    let action = match self.mode {
                        Mode::Game => self.emu.handle_key_event(key_event),
                        Mode::Settings => self.settings.handle_key_event(key_event),
                        Mode::RomSelection => self.finder.handle_key_event(key_event),
                        Mode::Menu => self.menu.handle_key_event(key_event),
                    };
                    if self.handle_action(action) {
                        needs_redraw = true;
                    }
                }
            }

            if self.should_quit {
                break 'main_loop;
            }
            let update_action = self.update();
            if self.handle_action(update_action) {
                needs_redraw = true;
            }
            if needs_redraw {
                self.render(&mut terminal);
            }
            let elapsed = self.metrics.end_frame(frame_start);
            if elapsed < target_frame_duration {
                std::thread::sleep(target_frame_duration - elapsed);
            }
        }
        let _ = self.finder.save();

        Ok(())
    }

    fn update(&mut self) -> Action {
        match &mut self.mode {
            Mode::Game => self.emu.update(),
            _ => Action::Nope,
        }
    }

    fn handle_events(&mut self) -> Action {
        let mut action = Action::Nope;
        // Polling the event if it is available
        if let Ok(one_evt) = crossterm::event::poll(Duration::from_millis(0)) {
            if one_evt {
                if let Ok(evt) = crossterm::event::read() {
                    if let Event::Key(key_evt) = evt {
                        if let KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        } = key_evt
                        {
                            self.should_quit = true;
                        } else {
                            match &mut self.mode {
                                Mode::Game => action = self.emu.handle_key_event(key_evt),
                                Mode::Settings => action = self.settings.handle_key_event(key_evt),
                                Mode::RomSelection => {
                                    action = self.finder.handle_key_event(key_evt)
                                }
                                Mode::Menu => action = self.menu.handle_key_event(key_evt),
                            }
                        }
                    }
                }
            }
        }
        action
    }

    fn switch_mode(&mut self, mode: Mode) {
        self.mode = mode;

        if let Mode::Game = self.mode {
            self.emu.config(&self.settings);
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        let mut needs_render = false;
        match action {
            Action::GoToSetting => self.switch_mode(Mode::Settings),
            Action::LoadRom(path) => {
                if self.emu.load_rom(&path) {
                    self.finder.register_rom(path);
                    self.switch_mode(Mode::Game);
                }
            }
            // Action::GoToGame => self.switch_mode(Mode::Game(Chip8::new())),
            Action::GoToRomFinder => self.switch_mode(Mode::RomSelection),
            Action::GoToMenu => self.switch_mode(Mode::Menu),
            Action::Quit => self.should_quit = true,
            Action::Render => {
                needs_render = true;
            }
            _ => (),
        }

        needs_render
    }

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        let _ = terminal.draw(|f| {
            match &mut self.mode {
                Mode::Menu => self.menu.render(f),
                Mode::RomSelection => self.finder.render(f),
                Mode::Game => self.emu.render(f),
                Mode::Settings => self.settings.render(f),
            }

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

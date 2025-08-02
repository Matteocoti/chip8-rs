use crate::actions::Action;
use crate::browser::RomFinder;
use crate::chip8_tui::Chip8TUI;
use crate::menu::MainMenu;
use crate::settings::Settings;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::{
    ExecutableCommand,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
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
    settings: Settings, // Application settings
    menu: MainMenu,     // Main menu component
    finder: RomFinder,  // Rom finder component
    emu: Chip8TUI,      // Emulator component
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
        Self {
            mode: Mode::Menu,
            should_quit: false,
            settings: Settings::new(),
            menu: MainMenu::new(),
            finder: RomFinder::new(),
            emu: Chip8TUI::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), color_eyre::Report> {
        let mut terminal = init_tui_terminal()?;

        let target_frame_duration = Duration::from_secs_f64(1.0 / 60.0);

        self.render(&mut terminal);
        loop {
            let start = std::time::Instant::now();
            let mut action = self.handle_events();
            self.handle_action(action, &mut terminal);
            if self.should_quit {
                let _ = restore_terminal();
                break;
            }
            action = self.update();
            self.handle_action(action, &mut terminal);
            let elapsed = start.elapsed();
            if elapsed < target_frame_duration {
                std::thread::sleep(target_frame_duration - elapsed);
            }
        }
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

    fn switch_mode(
        &mut self,
        mode: Mode,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) {
        self.mode = mode;

        if let Mode::Game = self.mode {
            self.emu.config(&self.settings);
        }

        self.render(terminal);
    }

    fn handle_action(
        &mut self,
        action: Action,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) {
        match action {
            Action::GoToSetting => self.switch_mode(Mode::Settings, terminal),
            Action::LoadRom(path) => {
                if self.emu.load_rom(path) {
                    self.switch_mode(Mode::Game, terminal);
                }
            }
            // Action::GoToGame => self.switch_mode(Mode::Game(Chip8::new())),
            Action::GoToRomFinder => self.switch_mode(Mode::RomSelection, terminal),
            Action::GoToMenu => self.switch_mode(Mode::Menu, terminal),
            Action::Quit => self.should_quit = true,
            Action::Render => self.render(terminal),
            _ => (),
        }
    }

    fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
        match &mut self.mode {
            Mode::Menu => self.menu.render(terminal),
            Mode::RomSelection => self.finder.render(terminal),
            Mode::Game => self.emu.render(terminal),
            Mode::Settings => self.settings.render(terminal),
        }
    }
}

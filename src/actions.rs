use std::path::PathBuf;

pub enum Action {
    Nope,
    LoadRom(PathBuf),
    GoToRomFinder,
    GoToGame,
    GoToSetting,
    GoToMenu,
    TogglePause,
    Render,
    Quit,
}

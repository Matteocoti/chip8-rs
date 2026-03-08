# chip8-rs

[![CI](https://github.com/Matteocoti/chip8-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/Matteocoti/chip8-rs/actions/workflows/rust.yml)

A CHIP-8 emulator written in Rust with a terminal UI built using [ratatui](https://github.com/ratatui/ratatui).

## Features

- Full CHIP-8 instruction set implementation
- Terminal UI with live debug panel (registers, stack, timers, PC, SP)
- ROM file browser and recent ROMs history
- Save/load states (multiple slots via timestamp)
- Configurable CPU frequency and max delta time
- Configurable key bindings
- Sine-wave audio via rodio
- Performance metrics overlay (FPS, frame time)
- Step-by-step execution mode for debugging

## Requirements

- Rust 1.85+ (edition 2024)
- A terminal with 256-color support
- Linux/macOS (uses crossterm; Windows is untested)

## Building

```bash
git clone https://github.com/you/chip8-rs
cd chip8-rs
cargo build --release
```

## Running

```bash
cargo run --release
```

The main menu lets you browse for a ROM or re-open a recently played one.

## Controls

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `↓` (or `w`/`s`, `k`/`j`) | Navigate menus |
| `Enter` | Select / open |
| `Esc` | Go back |
| `Tab` | Switch focus in split-pane views |
| `Ctrl+C` | Quit |

### In-game

| Key | Action |
|-----|--------|
| `Esc` | Return to main menu |
| `F1` | Increase CPU frequency |
| `F2` | Decrease CPU frequency |
| `F3` | Reset CPU frequency to default |
| `F4` | Reset / reload ROM |
| `F5` | Quick save state |
| `F6` | Quick load state (most recent save) |
| `Enter` | Toggle step mode |
| `n` | Step one cycle (while in step mode) |
| `F12` | Toggle performance metrics overlay |

### ROM file browser

| Key | Action |
|-----|--------|
| `→` / `Enter` | Open folder or load ROM |
| `←` | Go up one directory |
| `/` | Start filtering by name |
| `Ctrl+H` | Toggle hidden files |

### CHIP-8 Keypad (default mapping)

The original CHIP-8 hexadecimal keypad is mapped to QWERTY as follows:

```
CHIP-8    Keyboard
1 2 3 C   1 2 3 4
4 5 6 D   q w e r
7 8 9 E   a s d f
A 0 B F   z x c v
```

Key bindings can be reconfigured in the settings menu.

## Configuration

Settings are stored in `~/.chip8_tui/`:

| File | Contents |
|------|----------|
| `emulator_settings.toml` | CPU frequency (Hz), max delta time (ms) |
| `key_bindings.toml` | CHIP-8 keypad to keyboard mapping |
| `rom_history.toml` | Recently opened ROMs |
| `chip8.log` | Runtime log (ROM loads, errors, notifications) |
| `saved_data/<rom>/` | Save state files (`.sav`) |

### Emulator Settings

- **Frequency** — CPU cycles per second. Default: 500 Hz. Higher = faster emulation.
- **Max Delta Time** — caps the time delta per tick to avoid spiral-of-death on slow frames. Default: 30 ms.

## Architecture

The UI is built around a **component stack**. Each screen (menu, file browser, emulator) implements the `Component` trait. The active component is the top of the stack. Navigation uses `Action::Transition(Pop | Push | Switch)`.

The emulator core (`Chip8`) is timing-aware: `tick()` determines how many cycles to run based on elapsed wall-clock time and the configured CPU frequency.

## Running Tests

```bash
cargo test
```

150 tests cover all CHIP-8 opcodes, display bounds, memory read/write, input state, emulator settings, ROM history, save/load, and edge cases (overflow, wrapping, boundary conditions).

## License

MIT

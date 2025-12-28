# chip8-rs

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

The main menu lets you browse for a ROM, or re-open a recently played one.

You can also pass a ROM directly (if you wire up a CLI arg — currently the entry point opens the menu by default).

## Controls

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

- **Frequency** — CPU cycles per second. Default: 1000 Hz. Higher = faster emulation.
- **Max Delta Time** — caps the time delta per tick to avoid spiral-of-death on slow frames. Default: 30 ms.

## Project Structure

```
src/
  main.rs                  — entry point
  app.rs                   — main loop, component stack, event dispatch
  chip8_tui.rs             — TUI wrapper around the CHIP-8 core
  component.rs             — Component trait (handle_key_event, render, update, on_entry, on_exit)
  menu.rs                  — main menu
  file_browser.rs          — ROM file browser
  rom_history.rs           — recently opened ROMs
  audio.rs                 — rodio sine-wave audio handler
  performance_metrics.rs   — FPS / frame-time overlay
  config_manager.rs        — config directory paths
  config_file.rs           — config file helpers
  browser.rs               — generic list browser component
  split_view_component.rs  — split-pane layout helper
  constants.rs             — shared constants
  actions.rs               — action/event types (if separate from component.rs)
  settings/
    mod.rs
    emulator_settings.rs   — frequency + delta time settings
    key_bindings.rs        — key binding settings
    numeric_setting.rs     — generic numeric setting widget
    setting_item.rs        — SettingItem trait (typetag serde)
  chip8/
    mod.rs
    cpu.rs                 — Chip8 struct, tick loop, state save/load
    opcodes.rs             — all 35 CHIP-8 opcodes
    display.rs             — 64x32 frame buffer
    memory.rs              — 4 KB memory + font data
    input.rs               — 16-key keyboard state
    error.rs               — EmulationError type
```

## Architecture

The UI is built around a **component stack**. Each screen (menu, file browser, emulator) implements the `Component` trait. The active component is the top of the stack. Navigation uses `Action::Transition(Pop | Push | Switch)`.

The CHIP-8 core (`Chip8`) is timing-aware: `tick()` runs however many cycles fit in the elapsed wall-clock time, using a **burst mode** that keeps running extra cycles when a draw sequence is in progress. This prevents graphical artifacts (flickering / apparent freeze) when a ROM erases and redraws sprites across what would otherwise be multiple rendered frames.

## Running Tests

```bash
cargo test
```

Tests cover all CHIP-8 opcodes, display bounds checking, memory read/write, input state, emulator settings, and ROM history.

## License

MIT

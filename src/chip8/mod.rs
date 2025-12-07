// CHIP-8 emulator module exports

pub mod cpu;
pub mod display;
pub mod error;
pub mod input;
pub mod memory;
pub mod opcodes;

// Re-export main types
pub use cpu::Chip8;
pub use cpu::EmulationEvent;
pub use display::Chip8Display;
pub use error::EmulationError;
pub use input::Chip8Keyboard;
pub use memory::Chip8Memory;

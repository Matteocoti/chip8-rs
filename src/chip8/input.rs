// Input/keyboard handling for CHIP-8 emulator
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chip8Keyboard {
    keys: [bool; 16],
}

impl Default for Chip8Keyboard {
    fn default() -> Self {
        Self { keys: [false; 16] }
    }
}

impl Chip8Keyboard {
    pub fn new() -> Self {
        Chip8Keyboard::default()
    }

    pub fn clear(&mut self) {
        self.keys = [false; 16];
    }

    pub fn is_key_pressed(&self, key: u8) -> bool {
        if key < 16 {
            self.keys[key as usize]
        } else {
            false
        }
    }

    pub fn set_key(&mut self, key: u8, pressed: bool) {
        if key < 16 {
            self.keys[key as usize] = pressed;
        }
    }

    pub fn get_pressed_key(&self) -> Option<u8> {
        for (i, &pressed) in self.keys.iter().enumerate() {
            if pressed {
                return Some(i as u8);
            }
        }
        None
    }
}

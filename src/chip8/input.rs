// Input/keyboard handling for CHIP-8 emulator
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Chip8Keyboard {
    keys: [bool; 16],
}

impl Chip8Keyboard {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_no_keys_pressed() {
        let kb = Chip8Keyboard::default();
        for k in 0u8..16 {
            assert!(!kb.is_key_pressed(k));
        }
    }

    #[test]
    fn set_key_true_reports_pressed() {
        let mut kb = Chip8Keyboard::default();
        kb.set_key(0x5, true);
        assert!(kb.is_key_pressed(0x5));
    }

    #[test]
    fn set_key_false_releases_key() {
        let mut kb = Chip8Keyboard::default();
        kb.set_key(0x5, true);
        kb.set_key(0x5, false);
        assert!(!kb.is_key_pressed(0x5));
    }

    #[test]
    fn set_key_out_of_range_is_noop() {
        let mut kb = Chip8Keyboard::default();
        kb.set_key(16, true); // invalid — 16 is out of [0,15]
        // no valid key should have been pressed
        assert_eq!(kb.get_pressed_key(), None);
    }

    #[test]
    fn is_key_pressed_out_of_range_returns_false() {
        let kb = Chip8Keyboard::default();
        assert!(!kb.is_key_pressed(16));
        assert!(!kb.is_key_pressed(255));
    }

    #[test]
    fn get_pressed_key_returns_none_when_empty() {
        assert_eq!(Chip8Keyboard::default().get_pressed_key(), None);
    }

    #[test]
    fn get_pressed_key_returns_lowest_pressed_index() {
        let mut kb = Chip8Keyboard::default();
        kb.set_key(0xC, true);
        kb.set_key(0xA, true);
        assert_eq!(kb.get_pressed_key(), Some(0xA)); // lowest index first
    }

    #[test]
    fn clear_releases_all_keys() {
        let mut kb = Chip8Keyboard::default();
        for k in 0u8..16 {
            kb.set_key(k, true);
        }
        kb.clear();
        assert_eq!(kb.get_pressed_key(), None);
    }

    #[test]
    fn all_16_keys_can_be_pressed_independently() {
        let mut kb = Chip8Keyboard::default();
        for k in 0u8..16 {
            kb.set_key(k, true);
            assert!(kb.is_key_pressed(k));
            kb.set_key(k, false);
        }
    }
}

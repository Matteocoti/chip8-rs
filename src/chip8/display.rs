use crate::chip8::EmulationError;
use serde::{Deserialize, Deserializer, Serialize};

const DISPLAY_ROWS: usize = 32;
const DISPLAY_COLS: usize = 64;

#[derive(Clone, Debug)]
pub struct Chip8Display([bool; DISPLAY_COLS * DISPLAY_ROWS]);

impl Default for Chip8Display {
    fn default() -> Self {
        Self([false; DISPLAY_COLS * DISPLAY_ROWS])
    }
}

impl Serialize for Chip8Display {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes: Vec<u8> = self
            .0
            .chunks(8)
            .map(|chunk| {
                chunk.iter().enumerate().fold(
                    0u8,
                    |acc, (i, &bit)| {
                        if bit { acc | (1 << i) } else { acc }
                    },
                )
            })
            .collect();
        bytes.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Chip8Display {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut display = [false; 2048];

        for (byte_idx, &byte) in bytes.iter().enumerate() {
            for bit_idx in 0..8 {
                let array_idx = byte_idx * 8 + bit_idx;
                if array_idx < 2048 {
                    display[array_idx] = (byte & (1 << bit_idx)) != 0;
                }
            }
        }

        Ok(Self(display))
    }
}

impl Chip8Display {
    pub fn clear(&mut self) {
        self.0 = [false; DISPLAY_COLS * DISPLAY_ROWS];
    }

    pub fn pixel_state(&self, row: usize, column: usize) -> Result<bool, EmulationError> {
        if row >= DISPLAY_ROWS || column >= DISPLAY_COLS {
            return Err(EmulationError::DisplayLimit);
        }

        Ok(self.0[row * DISPLAY_COLS + column])
    }

    pub fn set_pixel_value(
        &mut self,
        row: usize,
        column: usize,
        value: bool,
    ) -> Result<(), EmulationError> {
        if row >= DISPLAY_ROWS || column >= DISPLAY_COLS {
            return Err(EmulationError::DisplayLimit);
        }

        self.0[row * DISPLAY_COLS + column] = value;
        Ok(())
    }

    pub fn get_frame_buffer(&self) -> &[bool] {
        self.0.as_slice()
    }

    pub fn get_num_rows(&self) -> usize {
        DISPLAY_ROWS
    }

    pub fn get_num_cols(&self) -> usize {
        DISPLAY_COLS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_all_pixels_false() {
        let display = Chip8Display::default();
        assert!(display.get_frame_buffer().iter().all(|&p| !p));
    }

    #[test]
    fn get_num_rows_is_32() {
        assert_eq!(Chip8Display::default().get_num_rows(), 32);
    }

    #[test]
    fn get_num_cols_is_64() {
        assert_eq!(Chip8Display::default().get_num_cols(), 64);
    }

    #[test]
    fn frame_buffer_length_is_2048() {
        assert_eq!(Chip8Display::default().get_frame_buffer().len(), 2048);
    }

    #[test]
    fn set_pixel_value_and_read_back() {
        let mut display = Chip8Display::default();
        display.set_pixel_value(5, 10, true).unwrap();
        assert!(display.pixel_state(5, 10).unwrap());
    }

    #[test]
    fn set_pixel_false_clears_pixel() {
        let mut display = Chip8Display::default();
        display.set_pixel_value(0, 0, true).unwrap();
        display.set_pixel_value(0, 0, false).unwrap();
        assert!(!display.pixel_state(0, 0).unwrap());
    }

    #[test]
    fn pixel_state_at_last_valid_position() {
        let mut display = Chip8Display::default();
        display.set_pixel_value(31, 63, true).unwrap();
        assert!(display.pixel_state(31, 63).unwrap());
    }

    #[test]
    fn pixel_state_row_out_of_bounds_returns_error() {
        assert!(Chip8Display::default().pixel_state(32, 0).is_err());
    }

    #[test]
    fn pixel_state_col_out_of_bounds_returns_error() {
        assert!(Chip8Display::default().pixel_state(0, 64).is_err());
    }

    #[test]
    fn set_pixel_row_out_of_bounds_returns_error() {
        assert!(
            Chip8Display::default()
                .set_pixel_value(32, 0, true)
                .is_err()
        );
    }

    #[test]
    fn set_pixel_col_out_of_bounds_returns_error() {
        assert!(
            Chip8Display::default()
                .set_pixel_value(0, 64, true)
                .is_err()
        );
    }

    #[test]
    fn clear_resets_all_pixels() {
        let mut display = Chip8Display::default();
        display.set_pixel_value(0, 0, true).unwrap();
        display.set_pixel_value(31, 63, true).unwrap();
        display.clear();
        assert!(display.get_frame_buffer().iter().all(|&p| !p));
    }

    #[test]
    fn frame_buffer_index_matches_row_col() {
        let mut display = Chip8Display::default();
        display.set_pixel_value(2, 5, true).unwrap();
        let fb = display.get_frame_buffer();
        assert!(fb[2 * 64 + 5]); // row 2, col 5
        assert!(!fb[2 * 64 + 6]); // adjacent pixel unset
    }
}

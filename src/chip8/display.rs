use crate::chip8::EmulationError;
use serde::{Deserialize, Deserializer, Serialize};

const DISPLAY_ROWS: usize = 32;
const DISPLAY_COLS: usize = 64;

#[derive(Clone, Debug)]
pub struct Chip8Display([bool; DISPLAY_COLS * DISPLAY_ROWS]);

impl Default for Chip8Display {
    fn default() -> Self {
        Self {
            0: [false; DISPLAY_COLS * DISPLAY_ROWS],
        }
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
        if row > DISPLAY_ROWS || column > DISPLAY_COLS {
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
        if row > DISPLAY_ROWS || column > DISPLAY_COLS {
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

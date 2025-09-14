use std::mem;

use crate::chip8::EmulationError;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone)]
pub struct Chip8Memory([u8; 4096]);

impl Default for Chip8Memory {
    fn default() -> Self {
        Self { 0: [0; 4096] }
    }
}

impl Serialize for Chip8Memory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes: Vec<u8> = self.0.to_vec();
        bytes.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Chip8Memory {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let bytes_array = bytes.try_into();

        Ok(Self(bytes_array.unwrap()))
    }
}

impl Chip8Memory {
    pub fn load_data(&mut self, start: usize, data: &[u8], len: usize) {
        self.0[start..(start + len)].copy_from_slice(data);
    }

    pub fn read_byte(&self, address: usize) -> Result<u8, EmulationError> {
        if (address) >= self.0.len() {
            return Err(EmulationError::InvalidAddress(address as u16));
        }

        Ok(self.0[address])
    }

    pub fn read_word(&self, address: usize) -> Result<u16, EmulationError> {
        let end_data = address + 2;

        if end_data >= self.0.len() {
            return Err(EmulationError::InvalidAddress(address as u16));
        }

        Ok(u16::from_be_bytes([self.0[address], self.0[address + 1]]))
    }

    pub fn set_byte(&mut self, address: usize, value: u8) -> Result<(), EmulationError> {
        if address >= self.0.len() {
            return Err(EmulationError::InvalidAddress(address as u16));
        }

        self.0[address] = value;
        Ok(())
    }

    pub fn size(&self) -> usize {
        mem::size_of_val(&self.0)
    }

    pub fn clear(&mut self) {
        self.0 = [0; 4096];
    }
}

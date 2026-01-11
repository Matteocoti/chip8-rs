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
        let bytes_array: [u8; 4096] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("memory must be exactly 4096 bytes"))?;

        Ok(Self(bytes_array))
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

        if end_data > self.0.len() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_zeros() {
        let mem = Chip8Memory::default();
        assert!(mem.0.iter().all(|&b| b == 0));
    }

    #[test]
    fn size_is_4096() {
        assert_eq!(Chip8Memory::default().size(), 4096);
    }

    #[test]
    fn load_data_and_read_back() {
        let mut mem = Chip8Memory::default();
        mem.load_data(0x200, &[0xAB, 0xCD, 0xEF], 3);
        assert_eq!(mem.read_byte(0x200).unwrap(), 0xAB);
        assert_eq!(mem.read_byte(0x201).unwrap(), 0xCD);
        assert_eq!(mem.read_byte(0x202).unwrap(), 0xEF);
    }

    #[test]
    fn read_byte_at_last_valid_address() {
        let mut mem = Chip8Memory::default();
        mem.load_data(4095, &[0xFF], 1);
        assert_eq!(mem.read_byte(4095).unwrap(), 0xFF);
    }

    #[test]
    fn read_byte_out_of_bounds_returns_error() {
        assert!(Chip8Memory::default().read_byte(4096).is_err());
    }

    #[test]
    fn read_word_returns_big_endian() {
        let mut mem = Chip8Memory::default();
        mem.load_data(0x200, &[0x12, 0x34], 2);
        assert_eq!(mem.read_word(0x200).unwrap(), 0x1234);
    }

    #[test]
    fn read_word_at_last_valid_address() {
        let mut mem = Chip8Memory::default();
        mem.load_data(4094, &[0xAB, 0xCD], 2);
        assert_eq!(mem.read_word(4094).unwrap(), 0xABCD);
    }

    #[test]
    fn read_word_spanning_past_end_returns_error() {
        // address 4095: only 1 byte remains, read_word needs 2
        assert!(Chip8Memory::default().read_word(4095).is_err());
    }

    #[test]
    fn set_byte_and_read_back() {
        let mut mem = Chip8Memory::default();
        mem.set_byte(0x300, 0x42).unwrap();
        assert_eq!(mem.read_byte(0x300).unwrap(), 0x42);
    }

    #[test]
    fn set_byte_out_of_bounds_returns_error() {
        assert!(Chip8Memory::default().set_byte(4096, 0xFF).is_err());
    }

    #[test]
    fn clear_zeroes_all_memory() {
        let mut mem = Chip8Memory::default();
        mem.load_data(0, &[0xFF; 100], 100);
        mem.clear();
        assert!(mem.0.iter().all(|&b| b == 0));
    }
}

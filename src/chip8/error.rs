use std::{error::Error, fmt};

// Custom error type for emulation errors.
#[derive(Debug)]
pub enum EmulationError {
    UnknownOpcode(u16),
    InvalidAddress(u16),
    DisplayLimit,
}

impl fmt::Display for EmulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmulationError::UnknownOpcode(opcode) => {
                write!(f, "Unknown opcode executed: 0x{:04X}", opcode)
            }
            EmulationError::InvalidAddress(address) => {
                write!(f, "Invalid Address: 0x{:04X}", address)
            }
            EmulationError::DisplayLimit => {
                write!(f, "Display index overflow!")
            }
        }
    }
}

impl Error for EmulationError {}

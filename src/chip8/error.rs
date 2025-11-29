use std::{error::Error, fmt};

// Custom error type for emulation errors.
#[derive(Debug)]
pub enum EmulationError {
    UnknownOpcode(u16),
    InvalidAddress(u16),
    DisplayLimit,
    StackOverflow,
    StackUnderflow,
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
            EmulationError::StackOverflow => write!(f, "Stack overflow"),
            EmulationError::StackUnderflow => write!(f, "Stack underflow"),
        }
    }
}

impl Error for EmulationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unknown_opcode() {
        assert_eq!(
            format!("{}", EmulationError::UnknownOpcode(0xABCD)),
            "Unknown opcode executed: 0xABCD"
        );
    }

    #[test]
    fn display_invalid_address() {
        assert_eq!(
            format!("{}", EmulationError::InvalidAddress(0x1234)),
            "Invalid Address: 0x1234"
        );
    }

    #[test]
    fn display_display_limit() {
        assert_eq!(
            format!("{}", EmulationError::DisplayLimit),
            "Display index overflow!"
        );
    }

    #[test]
    fn display_stack_overflow() {
        assert_eq!(
            format!("{}", EmulationError::StackOverflow),
            "Stack overflow"
        );
    }

    #[test]
    fn display_stack_underflow() {
        assert_eq!(
            format!("{}", EmulationError::StackUnderflow),
            "Stack underflow"
        );
    }
}

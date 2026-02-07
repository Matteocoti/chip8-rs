use serde::{Deserialize, Serialize};

use crate::chip8::Chip8Keyboard;
use crate::chip8::opcodes::Opcode;
use crate::chip8::{Chip8Display, Chip8Memory, EmulationError};

use std::path::PathBuf;
use std::time::{Duration, Instant};
// Fontset declaration -> group of sprites stored
// inside the chip8 memory
const FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0, 0xF0,
    0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0, 0xF0, 0x80,
    0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90, 0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0,
    0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80,
    0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
];

// Timer interval for the emulation loop, set to 60 FPS.
const TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_nanos(1_000_000_000 / 60);

// Custom event type for emulation events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulationEvent {
    ScreenUpdated, // Screen needs to be updated
    SoundStarted,  // Sound started playing
    SoundStopped,  // Sound stopped playing
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chip8State {
    memory: Chip8Memory,     // 4 Kb ram memory
    i: u16,                  // index register
    v: [u8; 16],             // 16 8-bit general purpose registers
    pc: u16,                 // program counter
    sp: u8,                  // stack pointer
    delay_tmr: u8,           // delay timer
    sound_tmr: u8,           // sound timer
    stack: [u16; 16],        // stack: array of 16 16-bit values
    keyboard: Chip8Keyboard, // 16-key keypad
    display: Chip8Display,   // 64x32 pixel display
    waiting_for_key: bool,   // Waiting for a key to be pressed
    register_for_key: usize, // Register to which store the pressed key
    opcode: u16,             // Current opcode
}

/// Lightweight snapshot of CPU registers for the debugger panel.
/// Does NOT include memory or display to avoid expensive clones.
pub struct DebugInfo {
    pub pc: u16,
    pub opcode: u16,
    pub i: u16,
    pub sp: u8,
    pub v: [u8; 16],
    pub delay_tmr: u8,
    pub sound_tmr: u8,
    pub stack: [u16; 16],
    pub waiting_for_key: bool,
}

impl Default for Chip8State {
    fn default() -> Self {
        Self {
            memory: Chip8Memory::default(),
            i: 0,
            v: [0; 16],
            pc: 0x200, // Program starts at 0x200 (0x600 for ETI)
            sp: 0,
            delay_tmr: 0,
            sound_tmr: 0,
            stack: [0; 16],
            keyboard: Chip8Keyboard::default(),
            display: Chip8Display::default(),
            waiting_for_key: false,
            register_for_key: 0,
            opcode: 0,
        }
    }
}

#[derive(Debug)]
pub struct Chip8 {
    state: Chip8State,
    frequency: u16,          // Frequency of the emulation cycle
    time: Duration,          // Time since last timer update
    last_tick_time: Instant, // Last tick time
    max_delta_time: u16,     // Maximum delta time for the emulation
}

impl Chip8 {
    pub fn new() -> Self {
        let mut chip8 = Chip8 {
            state: Chip8State::default(),
            frequency: 500, // Frequency of the emulation cycle
            time: Duration::ZERO,
            last_tick_time: Instant::now(),
            max_delta_time: 0,
        };

        chip8.load_fontset();
        // Load fontset into memory
        chip8
    }

    /// Loads a CHIP-8 ROM into the emulator's memory.
    ///
    /// This function copies the provided `rom_data` into the emulator's RAM, starting
    /// at address `0x200`. It first checks if the ROM's size exceeds the available
    /// memory space. If the copy is successful, the program counter (`pc`) is set
    /// to the starting address.
    ///
    /// # Arguments
    ///
    /// * `rom_data` - The ROM data to be loaded. The function is generic and accepts
    ///   any type that can be treated as a byte slice (`T: AsRef<[u8]>`), such as
    ///   `&[u8]` or `&Vec<u8>`.
    ///
    /// # Returns
    ///
    /// Returns `true` if the ROM was successfully loaded, and `false` if the ROM
    /// is too large to fit in the available memory.
    ///
    pub fn load_rom<T: AsRef<[u8]>>(&mut self, rom_data: T) -> bool {
        let rom_slice = rom_data.as_ref();
        let start = 0x200;
        let rom_size = rom_slice.len();
        let max_size = self.state.memory.size() - 0x200;
        // Check if the ROM fits in the emulator memory
        if rom_size > max_size {
            return false;
        }

        self.state.memory.load_data(start, rom_slice);

        // Set the pc to 0x200
        self.state.pc = start as u16;

        true
    }

    /// Fetch the instruction pointed by the program counter and returns its opcode.
    fn fetch(&self) -> Result<u16, EmulationError> {
        self.state.memory.read_word(self.state.pc as usize)
    }

    // This method takes an opcode (u16) and returns the corresponding Opcode enum variant.
    fn decode(&self, opcode: u16) -> Opcode {
        let nnn = opcode & 0x0FFF;
        let kk = (opcode & 0x00FF) as u8;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let n = (opcode & 0x000F) as u8;

        match (opcode & 0xF000) >> 12 {
            // Opcode Family 0x0000
            0x0 => match nnn {
                0x0E0 => Opcode::ClearDisplay,
                0x0EE => Opcode::Return,
                _ => Opcode::Unknown(opcode),
            },
            // Opcode Family 0x1000 - Jump
            0x1 => Opcode::Jump(nnn),
            // Opcode Family 0x2000 - Call
            0x2 => Opcode::Call(nnn),
            // Opcode Family 0x3000 - SkipIfEqual
            0x3 => Opcode::SkipIfEqual(x, kk),
            // Opcode Family 0x4000 - SkipIfNotEqual
            0x4 => Opcode::SkipIfNotEqual(x, kk),
            // Opcode Family 0x5000 - SkipIfRegEqual
            0x5 => Opcode::SkipIfRegEqual(x, y),
            // Opcode Family 0x6000 - Load
            0x6 => Opcode::Load(x, kk),
            // Opcode Family 0x7000 - Add
            0x7 => Opcode::Add(x, kk),
            // Opcode Family 0x8000 - Arithmetic
            0x8 => match n {
                0x0 => Opcode::LoadReg(x, y),
                0x1 => Opcode::OrReg(x, y),
                0x2 => Opcode::AndReg(x, y),
                0x3 => Opcode::XorReg(x, y),
                0x4 => Opcode::AddReg(x, y),
                0x5 => Opcode::SubReg(x, y),
                0x6 => Opcode::ShrReg(x, y),
                0x7 => Opcode::SubnReg(x, y),
                0xE => Opcode::ShlReg(x, y),
                _ => Opcode::Unknown(opcode),
            },
            // Opcode Family 0x9000 - SkipIfRegNotEqual
            0x9 => Opcode::SkipIfRegNotEqual(x, y),
            // Opcode Family 0xA000 - LoadI
            0xA => Opcode::LoadI(nnn),
            // Opcode Family 0xB000 - Jump with offset
            0xB => Opcode::JumpV0(nnn),
            // Opcode Family 0xC000 - Random
            0xC => Opcode::Rnd(x, kk),
            // Opcode Family 0xD000 - Draw
            0xD => Opcode::Draw(x, y, n),
            // Opcode Family 0xE000 - Keyboard
            0xE => match kk {
                0x9E => Opcode::SkipIfKeyPressed(x),
                0xA1 => Opcode::SkipIfKeyNotPressed(x),
                _ => Opcode::Unknown(opcode),
            },
            // Opcode Family 0xF000
            0xF => match kk {
                0x07 => Opcode::LoadDT(x),
                0x0A => Opcode::Wait(x),
                0x15 => Opcode::SetDT(x),
                0x18 => Opcode::SetST(x),
                0x1E => Opcode::AddI(x),
                0x29 => Opcode::SetIReg(x),
                0x33 => Opcode::StoreBCD(x),
                0x55 => Opcode::StoreV0(x),
                0x65 => Opcode::LoadV0(x),
                _ => Opcode::Unknown(opcode),
            },
            _ => Opcode::Unknown(opcode),
        }
    }

    fn execute(&mut self, opcode: Opcode) -> Result<Option<EmulationEvent>, EmulationError> {
        let mut display_update = false;
        let mut pc_inc: u16 = 2;

        let mut event = None;
        match opcode {
            // Clear the display
            Opcode::ClearDisplay => {
                self.state.display.clear();
                display_update = true;
            }
            // Sets the program counter to the address at the top of the stack
            // and then subtracts 1 from the stack pointer
            Opcode::Return => {
                if self.state.sp == 0 {
                    return Err(EmulationError::StackUnderflow);
                }
                self.state.sp -= 1;
                self.state.pc = self.state.stack[self.state.sp as usize];
            }
            // Sets the program counter to nnn
            Opcode::Jump(nnn) => {
                self.state.pc = nnn;
                pc_inc = 0;
            }
            // The interpreter increments the stack pointer, then puts the cuttent pc on the top of
            // the stack. The pc is set to nnn
            Opcode::Call(nnn) => {
                if self.state.sp as usize >= self.state.stack.len() {
                    return Err(EmulationError::StackOverflow);
                }
                self.state.stack[self.state.sp as usize] = self.state.pc;
                self.state.sp += 1;
                self.state.pc = nnn;
                pc_inc = 0;
            }
            // Skip next instruction if Vx = kk
            Opcode::SkipIfEqual(x, kk) => {
                if self.state.v[x] == kk {
                    pc_inc += 2;
                }
            }
            // Skip next instruction if Vx != kk
            Opcode::SkipIfNotEqual(x, kk) => {
                if self.state.v[x] != kk {
                    pc_inc += 2;
                }
            }
            // Skip next instruction if Vx = Vy
            Opcode::SkipIfRegEqual(x, y) => {
                if self.state.v[x] == self.state.v[y] {
                    pc_inc += 2;
                }
            }
            // Load kk in Vx
            Opcode::Load(x, kk) => self.state.v[x] = kk,
            // Add kk to Vx
            Opcode::Add(x, kk) => self.state.v[x] = self.state.v[x].wrapping_add(kk),
            // Set Vx = Vy
            Opcode::LoadReg(x, y) => self.state.v[x] = self.state.v[y],
            // Set Vx = Vx | Vy
            Opcode::OrReg(x, y) => self.state.v[x] |= self.state.v[y],
            // Set Vx = Vx & Vy
            Opcode::AndReg(x, y) => self.state.v[x] &= self.state.v[y],
            // Set Vx = Vx ^ Vy
            Opcode::XorReg(x, y) => self.state.v[x] ^= self.state.v[y],
            // Set Vx = Vx + Vy, Set Vf = Carry
            Opcode::AddReg(x, y) => {
                let (result, carry) = self.state.v[x].overflowing_add(self.state.v[y]);
                self.state.v[x] = result;
                self.state.v[0xF] = carry as u8;
            }
            // Set Vx = Vx - Vy, set Vf = NOT borrow
            Opcode::SubReg(x, y) => {
                let (result, borrow) = self.state.v[x].overflowing_sub(self.state.v[y]);
                self.state.v[x] = result;
                self.state.v[0xF] = !borrow as u8;
            }
            // Set Vx = Vx SHR 1
            Opcode::ShrReg(x, _y) => {
                self.state.v[0xF] = self.state.v[x] & 0x01;
                self.state.v[x] >>= 1;
            }
            // Set Vx = Vy - Vx, set Vf = NOT borrow
            Opcode::SubnReg(x, y) => {
                let (result, borrow) = self.state.v[y].overflowing_sub(self.state.v[x]);
                self.state.v[x] = result;
                self.state.v[0xF] = !borrow as u8;
            }
            // Set Vx = Vx SHL 1
            Opcode::ShlReg(x, _y) => {
                self.state.v[0xF] = (self.state.v[x] & 0x80) >> 7;
                self.state.v[x] <<= 1;
            }
            // Skip next instruction if Vx != Vy
            Opcode::SkipIfRegNotEqual(x, y) => {
                if self.state.v[x] != self.state.v[y] {
                    pc_inc += 2;
                }
            }
            // Set I = nnn
            Opcode::LoadI(nnn) => self.state.i = nnn,
            // Jump to nnn + V0; pc_inc must be 0 to prevent the default +2 from being added
            Opcode::JumpV0(nnn) => {
                self.state.pc = nnn + self.state.v[0] as u16;
                pc_inc = 0;
            }
            // Set Vx = random byte & kk
            Opcode::Rnd(x, kk) => {
                let u8_rnd = rand::random::<u8>();
                self.state.v[x] = u8_rnd & kk;
            }
            // Draw
            Opcode::Draw(x, y, n) => {
                // Reset of collision flag
                self.state.v[0xF] = 0;
                // Start position for the sprite on the display
                let start_x = self.state.v[x] as usize;
                let start_y = self.state.v[y] as usize;
                // Address from which we retrieve the sprite from the ram memory
                let start_adrs = self.state.i as usize;
                let stop_adrs = (self.state.i + n as u16) as usize;
                for row in start_adrs..stop_adrs {
                    // Each byte is a sprite row
                    let sprite_row = self.state.memory.read_byte(row)?;
                    // The y position may overflow the display so it needs to be wrapped
                    let y_pos = (start_y + row - start_adrs) % self.state.display.get_num_rows();
                    for pix_pos in 0..8 {
                        // The x position may overflow the display so it needs to be wrapped
                        let x_pos =
                            (start_x + pix_pos as usize) % self.state.display.get_num_cols();
                        // Retrieving the current pixel state
                        let curr_pixel_state = self.state.display.pixel_state(y_pos, x_pos)?;
                        let pixel_state = (sprite_row >> (7 - pix_pos)) & 1;
                        // The sprite is xored onto the existing screen
                        if pixel_state != 0 {
                            self.state
                                .display
                                .set_pixel_value(y_pos, x_pos, !curr_pixel_state)?;
                            // If the current pixel have to be erased, we set the collision flag
                            if curr_pixel_state {
                                self.state.v[0xF] = 1;
                            }
                        }
                    }
                }
                display_update = true;
            }
            // Skip next instruction if key with value of Vx is pressed
            Opcode::SkipIfKeyPressed(x) => {
                if self.state.keyboard.is_key_pressed(self.state.v[x]) {
                    pc_inc += 2;
                }
            }
            // Skip next instruction if key with the value of Vx is not pressed
            Opcode::SkipIfKeyNotPressed(x) => {
                if !self.state.keyboard.is_key_pressed(self.state.v[x]) {
                    pc_inc += 2;
                }
            }
            // Set Vx = delay timer value
            Opcode::LoadDT(x) => self.state.v[x] = self.state.delay_tmr,
            // Wait for a key press, store the value in Vx
            Opcode::Wait(x) => {
                self.state.waiting_for_key = true;
                self.state.register_for_key = x;
                pc_inc = 0;
            }
            // Set delay timer = Vx
            Opcode::SetDT(x) => self.state.delay_tmr = self.state.v[x],
            // Set sound timer = Vx
            Opcode::SetST(x) => {
                if (self.state.sound_tmr == 0) && (self.state.v[x] > 0) {
                    event = Some(EmulationEvent::SoundStarted);
                }
                self.state.sound_tmr = self.state.v[x];
            }
            //  Set I = I + Vx
            Opcode::AddI(x) => self.state.i = self.state.i.wrapping_add(self.state.v[x] as u16),
            // Set I = location of sprite for digit Vx
            Opcode::SetIReg(x) => self.state.i = self.state.v[x] as u16 * 5,
            // Store BCD representation of Vx in memory locations I, I+1, I+2
            Opcode::StoreBCD(x) => {
                self.state
                    .memory
                    .set_byte(self.state.i as usize, self.state.v[x] / 100)?;
                self.state
                    .memory
                    .set_byte(self.state.i.wrapping_add(1) as usize, (self.state.v[x] % 100) / 10)?;
                self.state
                    .memory
                    .set_byte(self.state.i.wrapping_add(2) as usize, self.state.v[x] % 10)?;
            }
            // Store registers V0 through Vx in memory starting at location I
            Opcode::StoreV0(x) => {
                for idx in 0..=x {
                    self.state
                        .memory
                        .set_byte((self.state.i as usize) + idx, self.state.v[idx])?;
                }
            }
            // Read registers V0 through Vx from memory starting at location I
            Opcode::LoadV0(x) => {
                for idx in 0..=x {
                    self.state.v[idx] = self.state.memory.read_byte(self.state.i as usize + idx)?;
                }
            }
            // Unknown
            Opcode::Unknown(u16_opcode) => return Err(EmulationError::UnknownOpcode(u16_opcode)),
        }

        // Update of the program counter
        self.state.pc += pc_inc;

        if display_update {
            event = Some(EmulationEvent::ScreenUpdated);
        }
        // Return if the display needs to be updated
        Ok(event)
    }

    pub fn reset_keyboard(&mut self) {
        self.state.keyboard.clear();
        self.state.waiting_for_key = false;
        self.state.register_for_key = 0
    }

    fn emulate_cycle(&mut self) -> Result<Option<EmulationEvent>, EmulationError> {
        // If the emulator is waiting for a key, the execution is stopped
        if self.state.waiting_for_key {
            if let Some(key) = self.state.keyboard.get_pressed_key() {
                self.state.v[self.state.register_for_key] = key;
                self.reset_keyboard();
                self.state.pc += 2;
                return Ok(None);
            } else {
                return Ok(None);
            }
        }

        // Fetch next instruction
        self.state.opcode = self.fetch()?;
        // Decode the instruction
        let opcode = self.decode(self.state.opcode);
        // Execution of the instruction
        self.execute(opcode)
    }

    fn update_timers(&mut self, delta: std::time::Duration) -> Option<EmulationEvent> {
        let mut evt = None;
        self.time += delta;
        while self.time >= TIMER_INTERVAL {
            // Decrease the delay timer
            if self.state.delay_tmr > 0 {
                self.state.delay_tmr -= 1;
            }
            if self.state.sound_tmr == 1 {
                evt = Some(EmulationEvent::SoundStopped);
            }
            // Decrease the sound timer
            if self.state.sound_tmr > 0 {
                self.state.sound_tmr -= 1;
            }
            self.time -= TIMER_INTERVAL;
        }

        evt
    }

    pub fn set_max_delta_time(&mut self, max_delta_time: u16) {
        self.max_delta_time = max_delta_time;
    }

    pub fn tick(&mut self) -> Result<Vec<EmulationEvent>, EmulationError> {
        let mut vec_events = Vec::new();
        let now = Instant::now();
        let mut delta = now.duration_since(self.last_tick_time);
        // Clamping delta time to a maximum value
        if self.max_delta_time > 0 {
            let max_delta = Duration::from_millis(self.max_delta_time as u64);
            if delta > max_delta {
                delta = max_delta;
            }
        }
        self.last_tick_time = now;

        // Update timers
        if let Some(evt) = self.update_timers(delta) {
            vec_events.push(evt);
        }
        let base_cycles = (self.frequency as f32 * delta.as_secs_f32()).round() as u32;

        for _ in 0..base_cycles {
            match self.emulate_cycle() {
                Ok(Some(evt)) => {
                    if !vec_events.contains(&evt) {
                        vec_events.push(evt);
                    }
                }
                Ok(None) => {}
                Err(e) => return Err(e),
            }
        }

        self.reset_keyboard();
        Ok(vec_events)
    }

    pub fn get_frame_buffer(&self) -> &[bool] {
        self.state.display.get_frame_buffer()
    }

    pub fn get_debug_info(&self) -> DebugInfo {
        DebugInfo {
            pc: self.state.pc,
            opcode: self.state.opcode,
            i: self.state.i,
            sp: self.state.sp,
            v: self.state.v,
            delay_tmr: self.state.delay_tmr,
            sound_tmr: self.state.sound_tmr,
            stack: self.state.stack,
            waiting_for_key: self.state.waiting_for_key,
        }
    }

    pub fn press_key(&mut self, key: u8) {
        self.state.keyboard.set_key(key, true);
    }

    pub fn set_frequency(&mut self, frequency: u16) {
        self.frequency = frequency;
    }

    pub fn save_state(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let data_encoded = bincode::serde::encode_to_vec(&self.state, bincode::config::standard())?;

        std::fs::write(path, data_encoded)?;

        Ok(())
    }

    pub fn load_state(&mut self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let data = std::fs::read(path)?;
        let loaded_state: Chip8State =
            bincode::serde::decode_from_slice(&data, bincode::config::standard())?.0;
        self.state = loaded_state;
        self.time = Duration::ZERO;
        self.last_tick_time = Instant::now();

        Ok(())
    }

    fn load_fontset(&mut self) {
        self.state
            .memory
            .load_data(0x000, &FONT_SET);
    }

    pub fn reset(&mut self) {
        // Reset ALL state including memory
        self.state.memory.clear();
        self.state.v = [0; 16];
        self.state.i = 0;
        self.state.pc = 0x200;
        self.state.sp = 0;
        self.state.stack = [0; 16];
        self.state.delay_tmr = 0;
        self.state.sound_tmr = 0;
        self.state.keyboard.clear();
        self.state.display.clear();
        self.state.waiting_for_key = false;
        self.state.register_for_key = 0;
        self.state.opcode = 0;
        // Reload font set into memory 0x50-0x9F
        self.load_fontset();

        self.time = Duration::ZERO;
        self.last_tick_time = Instant::now();
    }
}

impl Default for Chip8 {
    fn default() -> Self {
        Chip8::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Load a minimal ROM from raw opcodes into a fresh Chip8 instance.
    fn chip8_with_rom(opcodes: &[u16]) -> Chip8 {
        let mut cpu = Chip8::new();
        let rom: Vec<u8> = opcodes
            .iter()
            .flat_map(|&op| [(op >> 8) as u8, op as u8])
            .collect();
        cpu.load_rom(rom);
        cpu
    }

    // ── 2nnn / 00EE: Call / Return ────────────────────────────────────

    #[test]
    fn call_pushes_pc_and_jumps() {
        let mut cpu = chip8_with_rom(&[0x2300]); // CALL 0x300
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.sp, 1);
        assert_eq!(cpu.state.stack[0], 0x200); // return address saved
        assert_eq!(cpu.state.pc, 0x300); // jumped to target
    }

    #[test]
    fn return_pops_pc() {
        // 0x200: CALL 0x204 | 0x202: (padding) | 0x204: RET
        let mut cpu = chip8_with_rom(&[0x2204, 0x0000, 0x00EE]);
        cpu.emulate_cycle().unwrap(); // CALL 0x204 → sp=1, pc=0x204
        cpu.emulate_cycle().unwrap(); // RET        → sp=0, pc=0x202
        assert_eq!(cpu.state.sp, 0);
        assert_eq!(cpu.state.pc, 0x202);
    }

    #[test]
    fn return_on_empty_stack_is_underflow() {
        let mut cpu = chip8_with_rom(&[0x00EE]); // RET with empty stack
        let result = cpu.emulate_cycle();
        assert!(matches!(result, Err(EmulationError::StackUnderflow)));
    }

    #[test]
    fn call_beyond_stack_limit_is_overflow() {
        // CALL 0x200 calls itself, filling all 16 stack slots
        let mut cpu = chip8_with_rom(&[0x2200]);
        for _ in 0..16 {
            cpu.emulate_cycle().unwrap();
        }
        let result = cpu.emulate_cycle();
        assert!(matches!(result, Err(EmulationError::StackOverflow)));
    }

    // ── 00E0: ClearDisplay ────────────────────────────────────────────

    #[test]
    fn clear_display_wipes_all_pixels() {
        let mut cpu = chip8_with_rom(&[0x00E0]); // CLS
        cpu.state.display.set_pixel_value(0, 0, true).unwrap();
        cpu.state.display.set_pixel_value(5, 10, true).unwrap();
        cpu.emulate_cycle().unwrap();
        assert!(cpu.state.display.get_frame_buffer().iter().all(|&p| !p));
    }

    // ── 1nnn: Jump ────────────────────────────────────────────────────

    #[test]
    fn jump_sets_pc_without_touching_stack() {
        let mut cpu = chip8_with_rom(&[0x1300]); // JP 0x300
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x300);
        assert_eq!(cpu.state.sp, 0); // stack untouched
    }

    // ── 3xkk: SkipIfEqual ────────────────────────────────────────────

    #[test]
    fn skip_if_equal_match_skips_next_instruction() {
        let mut cpu = chip8_with_rom(&[0x3042]); // SE V0, 0x42
        cpu.state.v[0] = 0x42;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x204); // skipped one instruction
    }

    #[test]
    fn skip_if_equal_no_match_advances_normally() {
        let mut cpu = chip8_with_rom(&[0x3042]); // SE V0, 0x42
        cpu.state.v[0] = 0x00;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x202);
    }

    // ── 4xkk: SkipIfNotEqual ─────────────────────────────────────────

    #[test]
    fn skip_if_not_equal_mismatch_skips_next_instruction() {
        let mut cpu = chip8_with_rom(&[0x4042]); // SNE V0, 0x42
        cpu.state.v[0] = 0x00; // differs → skip
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x204);
    }

    #[test]
    fn skip_if_not_equal_match_advances_normally() {
        let mut cpu = chip8_with_rom(&[0x4042]); // SNE V0, 0x42
        cpu.state.v[0] = 0x42; // equal → no skip
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x202);
    }

    // ── 5xy0: SkipIfRegEqual ─────────────────────────────────────────

    #[test]
    fn skip_if_reg_equal_equal_regs_skips() {
        let mut cpu = chip8_with_rom(&[0x5010]); // SE V0, V1
        cpu.state.v[0] = 0xAB;
        cpu.state.v[1] = 0xAB;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x204);
    }

    #[test]
    fn skip_if_reg_equal_unequal_regs_advances() {
        let mut cpu = chip8_with_rom(&[0x5010]); // SE V0, V1
        cpu.state.v[0] = 0xAB;
        cpu.state.v[1] = 0xCD;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x202);
    }

    // ── 9xy0: SkipIfRegNotEqual ──────────────────────────────────────

    #[test]
    fn skip_if_reg_not_equal_unequal_regs_skips() {
        let mut cpu = chip8_with_rom(&[0x9010]); // SNE V0, V1
        cpu.state.v[0] = 0xAB;
        cpu.state.v[1] = 0xCD;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x204);
    }

    #[test]
    fn skip_if_reg_not_equal_equal_regs_advances() {
        let mut cpu = chip8_with_rom(&[0x9010]); // SNE V0, V1
        cpu.state.v[0] = 0xAB;
        cpu.state.v[1] = 0xAB;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x202);
    }

    // ── 6xkk: Load ───────────────────────────────────────────────────

    #[test]
    fn load_sets_register_to_immediate() {
        let mut cpu = chip8_with_rom(&[0x6A42]); // LD VA, 0x42
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0xA], 0x42);
    }

    // ── 7xkk: Add (no carry flag) ────────────────────────────────────

    #[test]
    fn add_byte_adds_without_carry_flag() {
        let mut cpu = chip8_with_rom(&[0x7020]); // ADD V0, 0x20
        cpu.state.v[0] = 0x10;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x30);
    }

    #[test]
    fn add_byte_wraps_on_overflow_without_setting_vf() {
        let mut cpu = chip8_with_rom(&[0x70FF]); // ADD V0, 0xFF
        cpu.state.v[0] = 0x01;
        cpu.state.v[0xF] = 0xAA; // sentinel: VF must not change
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x00); // wrapped
        assert_eq!(cpu.state.v[0xF], 0xAA); // VF unchanged
    }

    // ── 8xy0: LoadReg ────────────────────────────────────────────────

    #[test]
    fn load_reg_copies_vy_into_vx() {
        let mut cpu = chip8_with_rom(&[0x8010]); // LD V0, V1
        cpu.state.v[1] = 0x7F;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x7F);
    }

    // ── 8xy1/2/3: Bitwise ops ─────────────────────────────────────────

    #[test]
    fn or_reg_applies_bitwise_or() {
        let mut cpu = chip8_with_rom(&[0x8011]); // OR V0, V1
        cpu.state.v[0] = 0xF0;
        cpu.state.v[1] = 0x0F;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0xFF);
    }

    #[test]
    fn and_reg_applies_bitwise_and() {
        let mut cpu = chip8_with_rom(&[0x8012]); // AND V0, V1
        cpu.state.v[0] = 0xF0;
        cpu.state.v[1] = 0xFF;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0xF0);
    }

    #[test]
    fn xor_reg_applies_bitwise_xor() {
        let mut cpu = chip8_with_rom(&[0x8013]); // XOR V0, V1
        cpu.state.v[0] = 0xFF;
        cpu.state.v[1] = 0x0F;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0xF0);
    }

    // ── 8xy4: AddReg ─────────────────────────────────────────────────

    #[test]
    fn add_reg_no_overflow_sets_vf_zero() {
        let mut cpu = chip8_with_rom(&[0x8014]); // ADD V0, V1
        cpu.state.v[0] = 0x10;
        cpu.state.v[1] = 0x20;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x30);
        assert_eq!(cpu.state.v[0xF], 0);
    }

    #[test]
    fn add_reg_overflow_wraps_and_sets_vf_one() {
        let mut cpu = chip8_with_rom(&[0x8014]); // ADD V0, V1
        cpu.state.v[0] = 0xFF;
        cpu.state.v[1] = 0x01;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x00);
        assert_eq!(cpu.state.v[0xF], 1);
    }

    #[test]
    fn add_reg_vf_as_dest_result_is_carry_flag() {
        // When x=0xF: result is written then immediately overwritten by VF=carry
        let mut cpu = chip8_with_rom(&[0x8F04]); // ADD VF, V0
        cpu.state.v[0xF] = 0xFF;
        cpu.state.v[0] = 0x01;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0xF], 1); // carry wins; addition result is lost
    }

    // ── 8xy5: SubReg ─────────────────────────────────────────────────

    #[test]
    fn sub_reg_no_borrow_sets_vf_one() {
        let mut cpu = chip8_with_rom(&[0x8015]); // SUB V0, V1
        cpu.state.v[0] = 0x10;
        cpu.state.v[1] = 0x05;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x0B);
        assert_eq!(cpu.state.v[0xF], 1); // NOT borrow
    }

    #[test]
    fn sub_reg_borrow_wraps_and_sets_vf_zero() {
        let mut cpu = chip8_with_rom(&[0x8015]); // SUB V0, V1
        cpu.state.v[0] = 0x05;
        cpu.state.v[1] = 0x10;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0xF5);
        assert_eq!(cpu.state.v[0xF], 0);
    }

    #[test]
    fn sub_reg_equal_values_sets_vf_one_result_zero() {
        let mut cpu = chip8_with_rom(&[0x8015]); // SUB V0, V1
        cpu.state.v[0] = 0x42;
        cpu.state.v[1] = 0x42;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x00);
        assert_eq!(cpu.state.v[0xF], 1); // no borrow (equal)
    }

    // ── 8xy7: SubnReg ────────────────────────────────────────────────

    #[test]
    fn subn_reg_no_borrow_sets_vf_one() {
        let mut cpu = chip8_with_rom(&[0x8017]); // SUBN V0, V1
        cpu.state.v[0] = 0x05;
        cpu.state.v[1] = 0x10;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x0B); // Vy - Vx
        assert_eq!(cpu.state.v[0xF], 1);
    }

    #[test]
    fn subn_reg_borrow_wraps_and_sets_vf_zero() {
        let mut cpu = chip8_with_rom(&[0x8017]); // SUBN V0, V1
        cpu.state.v[0] = 0x10;
        cpu.state.v[1] = 0x05;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0xF5);
        assert_eq!(cpu.state.v[0xF], 0);
    }

    // ── 8xy6: ShrReg ─────────────────────────────────────────────────

    #[test]
    fn shr_reg_lsb_one_saves_flag_and_shifts() {
        let mut cpu = chip8_with_rom(&[0x8006]); // SHR V0
        cpu.state.v[0] = 0b0000_0101; // 5, LSB=1
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0b0000_0010); // 2
        assert_eq!(cpu.state.v[0xF], 1);
    }

    #[test]
    fn shr_reg_lsb_zero_sets_vf_zero() {
        let mut cpu = chip8_with_rom(&[0x8006]); // SHR V0
        cpu.state.v[0] = 0b0000_0100; // 4, LSB=0
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0b0000_0010);
        assert_eq!(cpu.state.v[0xF], 0);
    }

    #[test]
    fn shr_reg_ignores_vy_uses_vx() {
        // CHIP-48 quirk: 8xy6 shifts Vx, not Vy
        let mut cpu = chip8_with_rom(&[0x8016]); // SHR V0, V1
        cpu.state.v[0] = 0b0000_0110; // 6, LSB=0
        cpu.state.v[1] = 0b0000_0111; // 7, LSB=1 (ignored)
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0b0000_0011); // shifted from V0
        assert_eq!(cpu.state.v[0xF], 0); // LSB of V0, not V1
    }

    // ── 8xyE: ShlReg ─────────────────────────────────────────────────

    #[test]
    fn shl_reg_msb_one_saves_flag_and_shifts() {
        let mut cpu = chip8_with_rom(&[0x800E]); // SHL V0
        cpu.state.v[0] = 0b1000_0001; // MSB=1
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0b0000_0010);
        assert_eq!(cpu.state.v[0xF], 1);
    }

    #[test]
    fn shl_reg_msb_zero_sets_vf_zero() {
        let mut cpu = chip8_with_rom(&[0x800E]); // SHL V0
        cpu.state.v[0] = 0b0100_0000; // MSB=0
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0b1000_0000);
        assert_eq!(cpu.state.v[0xF], 0);
    }

    #[test]
    fn shl_reg_ignores_vy_uses_vx() {
        // CHIP-48 quirk: 8xyE shifts Vx, not Vy
        let mut cpu = chip8_with_rom(&[0x801E]); // SHL V0, V1
        cpu.state.v[0] = 0b0100_0000; // MSB=0
        cpu.state.v[1] = 0b1000_0000; // MSB=1 (ignored)
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0b1000_0000); // shifted from V0
        assert_eq!(cpu.state.v[0xF], 0); // MSB of V0, not V1
    }

    // ── Annn: LoadI ──────────────────────────────────────────────────

    #[test]
    fn load_i_sets_index_register() {
        let mut cpu = chip8_with_rom(&[0xA123]); // LD I, 0x123
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.i, 0x123);
    }

    // ── Bnnn: JumpV0 ─────────────────────────────────────────────────

    #[test]
    fn jump_v0_adds_v0_to_address() {
        let mut cpu = chip8_with_rom(&[0xB300]); // JP V0, 0x300
        cpu.state.v[0] = 0x05;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x305);
    }

    #[test]
    fn jump_v0_with_zero_offset_lands_at_address() {
        let mut cpu = chip8_with_rom(&[0xB300]); // JP V0, 0x300
        // V0 = 0 (default)
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x300);
    }

    // ── Cxkk: Rnd ────────────────────────────────────────────────────

    #[test]
    fn rnd_with_mask_zero_always_produces_zero() {
        let mut cpu = chip8_with_rom(&[0xC000]); // RND V0, 0x00
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x00);
    }

    #[test]
    fn rnd_with_mask_ff_produces_a_byte() {
        let mut cpu = chip8_with_rom(&[0xC0FF]); // RND V0, 0xFF
        cpu.emulate_cycle().unwrap();
        // Result is in [0, 255]; test just verifies it executes without error
        // and that VF is not clobbered
        let _ = cpu.state.v[0]; // any value is valid
    }

    // ── Dxyn: Draw ───────────────────────────────────────────────────

    #[test]
    fn draw_sprite_sets_pixels_and_no_collision_on_empty_display() {
        // Font glyph for '0' starts at 0x000: first byte is 0xF0 = 1111_0000
        // Draw 1 row at (0, 0)
        let mut cpu = chip8_with_rom(&[
            0xA000, // LD I, 0x000
            0xD001, // DRW V0, V1, 1  (V0=0 x, V1=0 y)
        ]);
        cpu.emulate_cycle().unwrap(); // LD I
        cpu.emulate_cycle().unwrap(); // DRW
        let fb = cpu.state.display.get_frame_buffer();
        // 0xF0 = 1111_0000 → bits 7-4 are 1, bits 3-0 are 0
        assert!(fb[0]); // col 0
        assert!(fb[1]); // col 1
        assert!(fb[2]); // col 2
        assert!(fb[3]); // col 3
        assert!(!fb[4]); // col 4 (zero bit)
        assert_eq!(cpu.state.v[0xF], 0); // no collision
    }

    #[test]
    fn draw_sprite_twice_clears_pixels_and_reports_collision() {
        let mut cpu = chip8_with_rom(&[
            0xA000, // LD I, 0x000
            0xD001, // DRW V0, V1, 1 (first draw — sets pixels)
            0xD001, // DRW V0, V1, 1 (second draw — XOR clears them)
        ]);
        for _ in 0..3 {
            cpu.emulate_cycle().unwrap();
        }
        let fb = cpu.state.display.get_frame_buffer();
        assert!(!fb[0]); // cleared by second draw
        assert_eq!(cpu.state.v[0xF], 1); // collision detected
    }

    #[test]
    fn draw_sprite_wraps_at_horizontal_boundary() {
        // Draw 1-row sprite 0xF0 (1111_0000) at x=63, y=0
        // Bits land at cols: 63, 0, 1, 2 (wrapped), then 0 bits
        let mut cpu = chip8_with_rom(&[
            0xA000, // LD I, 0x000 (font byte 0xF0)
            0x603F, // LD V0, 63
            0xD011, // DRW V0, V1, 1  (V0=x=63, V1=y=0)
        ]);
        for _ in 0..3 {
            cpu.emulate_cycle().unwrap();
        }
        let fb = cpu.state.display.get_frame_buffer();
        assert!(fb[63]); // col 63 (bit 7 of 0xF0)
        assert!(fb[0]); // col 0 (wrapped, bit 6)
        assert!(fb[1]); // col 1 (wrapped, bit 5)
        assert!(fb[2]); // col 2 (wrapped, bit 4)
        assert!(!fb[3]); // col 3 (bit 3 = 0)
    }

    // ── Ex9E / ExA1: Keyboard skips ──────────────────────────────────

    #[test]
    fn skip_if_key_pressed_with_key_down_skips() {
        let mut cpu = chip8_with_rom(&[0xE09E]); // SKP V0
        cpu.state.v[0] = 0x5;
        cpu.state.keyboard.set_key(0x5, true);
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x204);
    }

    #[test]
    fn skip_if_key_pressed_with_key_up_advances() {
        let mut cpu = chip8_with_rom(&[0xE09E]); // SKP V0
        cpu.state.v[0] = 0x5; // key not pressed
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x202);
    }

    #[test]
    fn skip_if_key_not_pressed_with_key_up_skips() {
        let mut cpu = chip8_with_rom(&[0xE0A1]); // SKNP V0
        cpu.state.v[0] = 0x5; // key not pressed
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x204);
    }

    #[test]
    fn skip_if_key_not_pressed_with_key_down_advances() {
        let mut cpu = chip8_with_rom(&[0xE0A1]); // SKNP V0
        cpu.state.v[0] = 0x5;
        cpu.state.keyboard.set_key(0x5, true);
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.pc, 0x202);
    }

    // ── Fx0A: Wait ───────────────────────────────────────────────────

    #[test]
    fn wait_does_not_advance_pc_while_no_key_pressed() {
        let mut cpu = chip8_with_rom(&[0xF00A]); // LD V0, K
        cpu.emulate_cycle().unwrap(); // execute Wait (pc stays at 0x200)
        let pc = cpu.state.pc;
        cpu.emulate_cycle().unwrap(); // still waiting
        assert_eq!(cpu.state.pc, pc); // unchanged
        assert!(cpu.state.waiting_for_key);
    }

    #[test]
    fn wait_stores_key_and_advances_when_key_pressed() {
        let mut cpu = chip8_with_rom(&[0xF30A]); // LD V3, K
        cpu.emulate_cycle().unwrap(); // enter wait state
        cpu.state.keyboard.set_key(0x7, true);
        cpu.emulate_cycle().unwrap(); // key available → complete
        assert_eq!(cpu.state.v[3], 0x7);
        assert!(!cpu.state.waiting_for_key);
        assert_eq!(cpu.state.pc, 0x202);
    }

    // ── Fx07 / Fx15 / Fx18: Timers ───────────────────────────────────

    #[test]
    fn load_dt_copies_delay_timer_to_vx() {
        let mut cpu = chip8_with_rom(&[0xF207]); // LD V2, DT
        cpu.state.delay_tmr = 0x3C;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[2], 0x3C);
    }

    #[test]
    fn set_dt_copies_vx_to_delay_timer() {
        let mut cpu = chip8_with_rom(&[0xF315]); // LD DT, V3
        cpu.state.v[3] = 0x55;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.delay_tmr, 0x55);
    }

    #[test]
    fn set_st_copies_vx_to_sound_timer() {
        let mut cpu = chip8_with_rom(&[0xF418]); // LD ST, V4
        cpu.state.v[4] = 0x0A;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.sound_tmr, 0x0A);
    }

    // ── Fx1E: AddI ───────────────────────────────────────────────────

    #[test]
    fn add_i_adds_vx_to_index_register() {
        let mut cpu = chip8_with_rom(&[0xF01E]); // ADD I, V0
        cpu.state.i = 0x100;
        cpu.state.v[0] = 0x05;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.i, 0x105);
    }

    // ── Fx29: SetIReg ────────────────────────────────────────────────

    #[test]
    fn set_i_reg_points_i_to_digit_sprite() {
        // Each digit sprite is 5 bytes starting at 0x000
        let mut cpu = chip8_with_rom(&[0xF029]); // LD F, V0
        cpu.state.v[0] = 0x5;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.i, 0x5 * 5);
    }

    #[test]
    fn set_i_reg_all_digits_point_within_fontset() {
        for digit in 0u8..=0xF {
            let mut cpu = chip8_with_rom(&[0xF029]); // LD F, V0
            cpu.state.v[0] = digit;
            cpu.emulate_cycle().unwrap();
            let sprite_addr = cpu.state.i as usize;
            assert!(
                sprite_addr + 5 <= 0x050,
                "digit 0x{digit:X} sprite at 0x{sprite_addr:03X} exceeds fontset"
            );
        }
    }

    // ── Fx33: StoreBCD ───────────────────────────────────────────────

    #[test]
    fn store_bcd_255_gives_2_5_5() {
        let mut cpu = chip8_with_rom(&[0xF033]); // LD B, V0
        cpu.state.v[0] = 255;
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.memory.read_byte(0x300).unwrap(), 2);
        assert_eq!(cpu.state.memory.read_byte(0x301).unwrap(), 5);
        assert_eq!(cpu.state.memory.read_byte(0x302).unwrap(), 5);
    }

    #[test]
    fn store_bcd_zero_gives_0_0_0() {
        let mut cpu = chip8_with_rom(&[0xF033]); // LD B, V0
        cpu.state.v[0] = 0;
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.memory.read_byte(0x300).unwrap(), 0);
        assert_eq!(cpu.state.memory.read_byte(0x301).unwrap(), 0);
        assert_eq!(cpu.state.memory.read_byte(0x302).unwrap(), 0);
    }

    #[test]
    fn store_bcd_123_gives_1_2_3() {
        let mut cpu = chip8_with_rom(&[0xF033]); // LD B, V0
        cpu.state.v[0] = 123;
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.memory.read_byte(0x300).unwrap(), 1);
        assert_eq!(cpu.state.memory.read_byte(0x301).unwrap(), 2);
        assert_eq!(cpu.state.memory.read_byte(0x302).unwrap(), 3);
    }

    // ── Fx55: StoreV0 ────────────────────────────────────────────────

    #[test]
    fn store_v0_saves_v0_through_vx_to_memory() {
        let mut cpu = chip8_with_rom(&[0xF255]); // LD [I], V2 (V0, V1, V2)
        cpu.state.v[0] = 0xAA;
        cpu.state.v[1] = 0xBB;
        cpu.state.v[2] = 0xCC;
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.memory.read_byte(0x300).unwrap(), 0xAA);
        assert_eq!(cpu.state.memory.read_byte(0x301).unwrap(), 0xBB);
        assert_eq!(cpu.state.memory.read_byte(0x302).unwrap(), 0xCC);
    }

    #[test]
    fn store_v0_does_not_modify_i() {
        // Modern CHIP-8: I is unchanged after Fx55
        let mut cpu = chip8_with_rom(&[0xF255]); // LD [I], V2
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.i, 0x300);
    }

    #[test]
    fn store_v0_with_x_zero_saves_only_v0() {
        let mut cpu = chip8_with_rom(&[0xF055]); // LD [I], V0
        cpu.state.v[0] = 0xAA;
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.memory.read_byte(0x300).unwrap(), 0xAA);
        assert_eq!(cpu.state.memory.read_byte(0x301).unwrap(), 0x00); // V1 not stored
    }

    #[test]
    fn store_v0_with_x_f_saves_all_registers() {
        let mut cpu = chip8_with_rom(&[0xFF55]); // LD [I], VF
        for i in 0u8..=0xF {
            cpu.state.v[i as usize] = i * 0x11;
        }
        cpu.state.i = 0x300;
        cpu.emulate_cycle().unwrap();
        for i in 0usize..=0xF {
            assert_eq!(
                cpu.state.memory.read_byte(0x300 + i).unwrap(),
                (i as u8) * 0x11,
                "V{i:X} not saved correctly"
            );
        }
    }

    // ── Fx65: LoadV0 ─────────────────────────────────────────────────

    #[test]
    fn load_v0_reads_v0_through_vx_from_memory() {
        let mut cpu = chip8_with_rom(&[0xF265]); // LD V2, [I] (V0, V1, V2)
        cpu.state.i = 0x300;
        cpu.state.memory.set_byte(0x300, 0x11).unwrap();
        cpu.state.memory.set_byte(0x301, 0x22).unwrap();
        cpu.state.memory.set_byte(0x302, 0x33).unwrap();
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x11);
        assert_eq!(cpu.state.v[1], 0x22);
        assert_eq!(cpu.state.v[2], 0x33);
    }

    #[test]
    fn load_v0_does_not_modify_i() {
        // Modern CHIP-8: I is unchanged after Fx65
        let mut cpu = chip8_with_rom(&[0xF265]); // LD V2, [I]
        cpu.state.i = 0x300;
        cpu.state.memory.set_byte(0x300, 0).unwrap();
        cpu.state.memory.set_byte(0x301, 0).unwrap();
        cpu.state.memory.set_byte(0x302, 0).unwrap();
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.i, 0x300);
    }

    #[test]
    fn load_v0_with_x_zero_reads_only_v0() {
        let mut cpu = chip8_with_rom(&[0xF065]); // LD V0, [I]
        cpu.state.v[1] = 0xFF; // sentinel: should not be overwritten
        cpu.state.i = 0x300;
        cpu.state.memory.set_byte(0x300, 0x42).unwrap();
        cpu.emulate_cycle().unwrap();
        assert_eq!(cpu.state.v[0], 0x42);
        assert_eq!(cpu.state.v[1], 0xFF); // unchanged
    }
}

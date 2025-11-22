use serde::{Deserialize, Serialize};

use crate::chip8::Chip8Keyboard;
use crate::chip8::opcodes::Opcode;
use crate::chip8::{Chip8Display, Chip8Memory, EmulationError};

use std::fmt;
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
    v: [u8; 16],             // 16 8-it general purpose register
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

impl fmt::Display for Chip8State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Scrive i registri principali
        writeln!(
            f,
            "PC: 0x{:04X}  |  I: 0x{:04X}  |  SP: 0x{:02X}",
            self.pc, self.i, self.sp
        )?;
        writeln!(f, "Opcode: 0x{:04X}", self.opcode)?;
        writeln!(f, "------------------------------------")?;

        write!(f, "V: ")?;
        for i in 0..8 {
            write!(f, "V{:X}:0x{:02X} ", i, self.v[i])?;
            writeln!(f)?;
        }

        for i in 8..16 {
            write!(f, "V{:X}:0x{:02X} ", i, self.v[i])?;
            writeln!(f)?;
        }
        writeln!(f, "------------------------------------")?;

        writeln!(
            f,
            "Delay Timer: {} | Sound Timer: {}",
            self.delay_tmr, self.sound_tmr
        )?;

        Ok(())
    }
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
            println!("Error: ROM data size too big!");
            return false;
        }

        self.state.memory.load_data(start, rom_slice, rom_size);

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
            // and then substracts 1 from the stack pointer
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
            Opcode::AddI(x) => self.state.i += self.state.v[x] as u16,
            // Set I = location of sprite for digit Vx
            Opcode::SetIReg(x) => self.state.i = self.state.v[x] as u16 * 5,
            // Store BCD representation of Vx in memory locations I, I+1, I+2
            Opcode::StoreBCD(x) => {
                self.state
                    .memory
                    .set_byte(self.state.i as usize, self.state.v[x] / 100)?;
                self.state
                    .memory
                    .set_byte((self.state.i + 1) as usize, (self.state.v[x] % 100) / 10)?;
                self.state
                    .memory
                    .set_byte((self.state.i + 2) as usize, self.state.v[x] % 10)?;
            }
            // Store registers V0 through Vx in memory starting at location I
            Opcode::StoreV0(x) => {
                for idx in 0..=x {
                    self.state
                        .memory
                        .set_byte((self.state.i as usize) + idx, self.state.v[idx])?;
                }
            }
            // Read registers V0 through Vx from memoty starting at location I
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

    #[allow(dead_code)]
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
        // Emulate a cycle
        let cycles_to_emulate = (self.frequency as f32 * delta.as_secs_f32()).round() as u32;
        for _ in 0..cycles_to_emulate {
            // Emulate a cycle
            if let Ok(cycle_evt) = self.emulate_cycle() {
                if let Some(evt) = cycle_evt {
                    if !vec_events.contains(&evt) {
                        vec_events.push(evt);
                    }
                }
            } else {
                return Err(EmulationError::UnknownOpcode(self.state.opcode));
            }
        }

        self.reset_keyboard();
        Ok(vec_events)
    }

    pub fn get_frame_buffer(&self) -> &[bool] {
        self.state.display.get_frame_buffer()
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

    pub fn get_state(&self) -> Chip8State {
        self.state.clone()
    }

    fn load_fontset(&mut self) {
        self.state
            .memory
            .load_data(0x000, &FONT_SET, FONT_SET.len());
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

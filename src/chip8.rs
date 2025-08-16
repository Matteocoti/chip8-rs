use std::error::Error;
use std::fmt;
use std::mem;
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

// Custom error type for emulation errors.
#[derive(Debug)]
pub enum EmulationError {
    UnknownOpcode(u16),
}

impl fmt::Display for EmulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmulationError::UnknownOpcode(opcode) => {
                write!(f, "Unknown opcode executed: 0x{:04X}", opcode)
            }
        }
    }
}

impl Error for EmulationError {}

enum Opcode {
    ClearDisplay,                    // 00E0
    Return,                          // 00EE
    Jump(u16),                       // 1nnn
    Call(u16),                       // 2nnn
    SkipIfEqual(usize, u8),          // 3xkk
    SkipIfNotEqual(usize, u8),       // 4xkk
    SkipIfRegEqual(usize, usize),    // 5xy0
    Load(usize, u8),                 // 6xkk
    Add(usize, u8),                  // 7xkk
    LoadReg(usize, usize),           // 8xy0
    OrReg(usize, usize),             // 8xy1
    AndReg(usize, usize),            // 8xy2
    XorReg(usize, usize),            // 8xy3
    AddReg(usize, usize),            // 8xy4
    SubReg(usize, usize),            // 8xy5
    ShrReg(usize, usize),            // 8xy6
    SubnReg(usize, usize),           // 8xy7
    ShlReg(usize, usize),            // 8xyE
    SkipIfRegNotEqual(usize, usize), // 9xy0
    LoadI(u16),                      // Annn
    JumpV0(u16),                     // Bnnn
    Rnd(usize, u8),                  // Cxkk
    Draw(usize, usize, u8),          // Dxyn
    SkipIfKeyPressed(usize),         // Ex9E
    SkipIfKeyNotPressed(usize),      // ExA1
    LoadDT(usize),                   // Fx07
    Wait(usize),                     // Fx0A
    SetDT(usize),                    // Fx15
    SetST(usize),                    // Fx18
    AddI(usize),                     // Fx1E
    SetIReg(usize),                  // Fx29
    StoreBCD(usize),                 // Fx33
    StoreV0(usize),                  // Fx55
    LoadV0(usize),                   // Fx65
    Unknown(u16),                    // Unknown opcode
}

#[derive(Debug)]
pub struct State {
    pub i: u16,        // index register
    pub v: [u8; 16],   // 16 8-it general purpose register
    pub pc: u16,       // program counter
    pub sp: u8,        // stack pointer
    pub delay_tmr: u8, // delay timer
    pub sound_tmr: u8, // sound timer
    pub code: u16,     // current opcode
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Scrive i registri principali
        writeln!(
            f,
            "PC: 0x{:04X}  |  I: 0x{:04X}  |  SP: 0x{:02X}",
            self.pc, self.i, self.sp
        )?;
        writeln!(f, "Opcode: 0x{:04X}", self.code)?;
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

#[derive(Debug)]
pub struct Chip8 {
    memory: [u8; 4096],       // 4 KB ram memory
    i: u16,                   // index register
    v: [u8; 16],              // 16 8-it general purpose register
    pc: u16,                  // program counter
    sp: u8,                   // stack pointer
    delay_tmr: u8,            // delay timer
    sound_tmr: u8,            // sound timer
    stack: [u16; 16],         // stack: array of 16 16-bit values
    keyboard: [bool; 16],     // 16-key keypad
    display: [bool; 64 * 32], // 64x32 pixel display
    waiting_for_key: bool,    // Waiting for a key to be pressed
    register_for_key: usize,  // Register to which store the pressed key
    opcode: u16,              // Current opcode
    frequency: u16,           // Frequency of the emulation cycle
    time: Duration,           // Time since last timer update
    last_tick_time: Instant,  // Last tick time
}

impl Chip8 {
    pub fn new() -> Self {
        let mut chip8 = Chip8 {
            memory: [0; 4096],
            i: 0,
            v: [0; 16],
            pc: 0x200, // Program starts at 0x200 (0x600 for ETI)
            sp: 0,
            delay_tmr: 0,
            sound_tmr: 0,
            stack: [0; 16],
            keyboard: [false; 16],
            display: [false; 64 * 32],
            waiting_for_key: false,
            register_for_key: 0,
            opcode: 0,
            frequency: 500, // Frequency of the emulation cycle
            time: Duration::ZERO,
            last_tick_time: Instant::now(),
        };

        // Load fontset into memory
        chip8.memory[0x000..0x050].copy_from_slice(&FONT_SET);

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
        let max_size = mem::size_of_val(&self.memory) - 0x200;
        // Check if the ROM fits in the emulator memory
        if rom_size > max_size {
            println!("Error: ROM data size too big!");
            return false;
        }

        let end = start + rom_size;
        self.memory[start..end].copy_from_slice(rom_slice);

        // Set the pc to 0x200
        self.pc = start as u16;

        true
    }

    /// Fetch the instruction pointed by the program counter and returns its opcode.
    fn fetch(&self) -> u16 {
        u16::from_be_bytes(
            self.memory[self.pc as usize..self.pc as usize + 2]
                .try_into()
                .unwrap(),
        )
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

    fn execute(&mut self, opcode: Opcode) -> Result<bool, EmulationError> {
        let mut display_update = false;
        let mut pc_inc: u16 = 2;

        match opcode {
            // Clear the display
            Opcode::ClearDisplay => {
                self.display = [false; 64 * 32];
                display_update = true;
            }
            // Sets the program counter to the address at the top of the stack
            // and then substracts 1 from the stack pointer
            Opcode::Return => {
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            // Sets the program counter to nnn
            Opcode::Jump(nnn) => {
                self.pc = nnn;
                pc_inc = 0;
            }
            // The interpreter increments the stack pointer, then puts the cuttent pc on the top of
            // the stack. The pc is set to nnn
            Opcode::Call(nnn) => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
                pc_inc = 0;
            }
            // Skip next instruction if Vx = kk
            Opcode::SkipIfEqual(x, kk) => {
                if self.v[x] == kk {
                    pc_inc += 2;
                }
            }
            // Skip next instruction if Vx != kk
            Opcode::SkipIfNotEqual(x, kk) => {
                if self.v[x] != kk {
                    pc_inc += 2;
                }
            }
            // Skip next instruction if Vx = Vy
            Opcode::SkipIfRegEqual(x, y) => {
                if self.v[x] == self.v[y] {
                    pc_inc += 2;
                }
            }
            // Load kk in Vx
            Opcode::Load(x, kk) => self.v[x] = kk,
            // Add kk to Vx
            Opcode::Add(x, kk) => self.v[x] = self.v[x].wrapping_add(kk),
            // Set Vx = Vy
            Opcode::LoadReg(x, y) => self.v[x] = self.v[y],
            // Set Vx = Vx | Vy
            Opcode::OrReg(x, y) => self.v[x] |= self.v[y],
            // Set Vx = Vx & Vy
            Opcode::AndReg(x, y) => self.v[x] &= self.v[y],
            // Set Vx = Vx ^ Vy
            Opcode::XorReg(x, y) => self.v[x] ^= self.v[y],
            // Set Vx = Vx + Vy, Set Vf = Carry
            Opcode::AddReg(x, y) => {
                let (result, carry) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = result;
                self.v[0xF] = carry as u8;
            }
            // Set Vx = Vx - Vy, set Vf = NOT borrow
            Opcode::SubReg(x, y) => {
                let (result, borrow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = result;
                self.v[0xF] = !borrow as u8;
            }
            // Set Vx = Vx SHR 1
            Opcode::ShrReg(x, _y) => {
                self.v[0xF] = self.v[x] & 0x01;
                self.v[x] >>= 1;
            }
            // Set Vx = Vy - Vx, set Vf = NOT borrow
            Opcode::SubnReg(x, y) => {
                let (result, borrow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = result;
                self.v[0xF] = !borrow as u8;
            }
            // Set Vx = Vx SHL 1
            Opcode::ShlReg(x, _y) => {
                self.v[0xF] = (self.v[x] & 0x80) >> 7;
                self.v[x] <<= 1;
            }
            // Skip next instruction if Vx != Vy
            Opcode::SkipIfRegNotEqual(x, y) => {
                if self.v[x] != self.v[y] {
                    pc_inc += 2;
                }
            }
            // Set I = nnn
            Opcode::LoadI(nnn) => self.i = nnn,
            // Jump to nnn + V0
            Opcode::JumpV0(nnn) => self.pc = nnn + self.v[0] as u16,
            // Set Vx = random byte & kk
            Opcode::Rnd(x, kk) => {
                let u8_rnd = rand::random::<u8>();
                self.v[x] = u8_rnd & kk;
            }
            // Draw
            Opcode::Draw(x, y, n) => {
                // Reset of collision flag
                self.v[0xF] = 0;
                // Start position for the sprite on the display
                let start_x = self.v[x] as usize;
                let start_y = self.v[y] as usize;
                // Address from which we retrieve the sprite from the ram memory
                let start_adrs = self.i as usize;
                let stop_adrs = (self.i + n as u16) as usize;
                for row in start_adrs..stop_adrs {
                    // Each byte is a sprite row
                    let sprite_row = self.memory[row];
                    // The y position may overflow the display so it needs to be wrapped
                    let y_pos = (start_y + row - start_adrs) % 32;
                    for pix_pos in 0..8 {
                        // The x position may overflow the display so it needs to be wrapped
                        let x_pos = (start_x + pix_pos as usize) % 64;
                        let pixel_idx = x_pos + y_pos * 64;
                        // Retrieving the current pixel state
                        let curr_pixel_state = self.display[pixel_idx];
                        let pixel_state = (sprite_row >> (7 - pix_pos)) & 1;
                        // The sprite is xored onto the existing screen
                        if pixel_state != 0 {
                            self.display[pixel_idx] = !curr_pixel_state;
                            // If the current pixel have to be erased, we set the collision flag
                            if curr_pixel_state {
                                self.v[0xF] = 1;
                            }
                        }
                    }
                }
                display_update = true;
            }
            // Skip next instruction if key with value of Vx is pressed
            Opcode::SkipIfKeyPressed(x) => {
                if self.keyboard[self.v[x] as usize] {
                    pc_inc += 2;
                }
            }
            // Skip next instruction if key with the value of Vx is not pressed
            Opcode::SkipIfKeyNotPressed(x) => {
                if !self.keyboard[self.v[x] as usize] {
                    pc_inc += 2;
                }
            }
            // Set Vx = delay timer value
            Opcode::LoadDT(x) => self.v[x] = self.delay_tmr,
            // Wait for a key press, store the value in Vx
            Opcode::Wait(x) => {
                self.waiting_for_key = true;
                self.register_for_key = x;
                pc_inc = 0;
            }
            // Set delay timer = Vx
            Opcode::SetDT(x) => self.delay_tmr = self.v[x],
            // Set sound timer = Vx
            Opcode::SetST(x) => self.sound_tmr = self.v[x],
            //  Set I = I + Vx
            Opcode::AddI(x) => self.i += self.v[x] as u16,
            // Set I = location of sprite for digit Vx
            Opcode::SetIReg(x) => self.i = self.v[x] as u16 * 5,
            // Store BCD representation of Vx in memory locations I, I+1, I+2
            Opcode::StoreBCD(x) => {
                self.memory[self.i as usize] = self.v[x] / 100;
                self.memory[(self.i + 1) as usize] = (self.v[x] % 100) / 10;
                self.memory[(self.i + 2) as usize] = self.v[x] % 10;
            }
            // Store registers V0 through Vx in memory starting at location I
            Opcode::StoreV0(x) => {
                for idx in 0..=x {
                    self.memory[self.i as usize + idx] = self.v[idx];
                }
            }
            // Read registers V0 through Vx from memoty starting at location I
            Opcode::LoadV0(x) => {
                for idx in 0..=x {
                    self.v[idx] = self.memory[self.i as usize + idx];
                }
            }
            // Unknown
            Opcode::Unknown(u16_opcode) => return Err(EmulationError::UnknownOpcode(u16_opcode)),
        }

        // Update of the program counter
        self.pc += pc_inc;

        // Return if the display needs to be updated
        Ok(display_update)
    }

    fn get_key_pressed(&self) -> Option<usize> {
        self.keyboard.iter().position(|&x| x)
    }

    pub fn reset_keyboard(&mut self) {
        self.keyboard = [false; 16];
        self.waiting_for_key = false;
        self.register_for_key = 0
    }

    fn emulate_cycle(&mut self) -> Result<bool, EmulationError> {
        let mut screen_update = false;
        // If the emulator is waiting for a key, the execution is stopped
        if self.waiting_for_key {
            if let Some(key) = self.get_key_pressed() {
                self.v[self.register_for_key] = key as u8;
                self.reset_keyboard();
                self.pc += 2;
                return Ok(false);
            } else {
                return Ok(false);
            }
        }

        // Fetch next instruction
        self.opcode = self.fetch();
        // Decode the instruction
        let opcode = self.decode(self.opcode);
        // Execution of the instruction
        if let Ok(update) = self.execute(opcode) {
            screen_update |= update;
        }
        Ok(screen_update)
    }

    fn update_timers(&mut self, delta: std::time::Duration) {
        self.time += delta;

        while self.time >= TIMER_INTERVAL {
            // Decrease the delay timer
            if self.delay_tmr > 0 {
                self.delay_tmr -= 1;
            }
            // Decrease the sound timer
            if self.sound_tmr > 0 {
                self.sound_tmr -= 1;
            }
            self.time -= TIMER_INTERVAL;
        }
    }

    pub fn tick(&mut self) -> Result<bool, EmulationError> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_tick_time);
        self.last_tick_time = now;
        // Update timers
        self.update_timers(delta);
        // Emulate a cycle
        let cycles_to_emulate = (self.frequency as f32 * delta.as_secs_f32()).round() as u32;
        let mut screen_update = false;
        for _ in 0..cycles_to_emulate {
            // Emulate a cycle
            if let Ok(update) = self.emulate_cycle() {
                screen_update |= update;
            } else {
                return Err(EmulationError::UnknownOpcode(self.opcode));
            }
        }
        Ok(screen_update)
    }

    pub fn get_frame_buffer(&self) -> &[bool] {
        &self.display
    }

    pub fn press_key(&mut self, key: u8) {
        self.keyboard[key as usize] = true;
    }

    pub fn get_state(&self) -> State {
        State {
            i: self.i,
            v: self.v,
            delay_tmr: self.delay_tmr,
            sound_tmr: self.sound_tmr,
            code: self.opcode,
            pc: self.pc,
            sp: self.sp,
        }
    }
}

impl Default for Chip8 {
    fn default() -> Self {
        Chip8::new()
    }
}

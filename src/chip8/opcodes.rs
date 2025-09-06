pub enum Opcode {
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

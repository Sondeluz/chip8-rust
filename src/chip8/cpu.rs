/// This is pretty much based on https://github.com/starrhorne/chip8-rust and
/// https://en.wikipedia.org/wiki/CHIP-8#Opcode_table, with a couple renamings
/// and a few instruction rewrites.

use crate::chip8::graphics::Graphics;
use crate::chip8::keypad::Keypad;

use rand::Rng;
use std::fs::File;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;

use crate::config;

/// Memory layout, registers(v), stack and graphics_subsystem matrix
pub struct Cpu<'a> {
    memory : [u8; 4096],
    v : [u8; 16], //V0 - VF, where VF doubles as a flag for some instructions (carry flag)
    i : usize, // I, limited to 12 bits / 0xFFF
    pc : usize, // Needs to be usize (8 bytes in x86_64) in order to index slices, limited to 12 bits / 0xFFF
    timers : Arc<Mutex<(u8, u8)>>, // (delay_timer, sound_timer), behind a shared mutex, since the timer thread updates them
    pause : Rc<RefCell<bool>>, // shared pause flag, triggered by the keypad subsystem
    // Instead of using a stack and a stack pointer, 
    // we can simply use a Vec and push()/pop() values
    // although we lose the sense of using a limited
    // stack and a SP
    stack : Vec<usize>, // limited to 12 bits / 0xFFF

    // Pointers to subsystems
    graphics_subsystem : Box<Graphics<'a>>,
    keypad_subsystem : Box<Keypad>,

    wants_to_quit : bool, // Signals that we have to exit the VM,
    instr_log : Vec<u16>,   // Instruction log for the display, this could be done with a normal array but we don't need
                            // it to be fast

    // Options
    config : &'a config::Config
}

/// Indicates the next value the PC is going to have, depending on the result of an instruction
enum NextPCValue {
    Next,
    Skip,
    Jump(usize),
}

impl Cpu<'_> {
    pub fn new<'a>(sdl_context : &'a sdl2::Sdl, config : &'a config::Config, timers : Arc<Mutex<(u8, u8)>>, pause : Rc<RefCell<bool>>, freq_period : Rc<RefCell<u64>>, ttf_context : sdl2::ttf::Sdl2TtfContext) -> Cpu<'a> {
        // Pre-allocate fonts in the reserved space (0x000 to 0x199)
        let mut temp_memory : [u8; 4096] = [0; 4096]; 
        
        Cpu::load_fonts(&mut temp_memory);
        Cpu::load_rom(config.rom_path(), &mut temp_memory);
    
        let pause_inner = Rc::clone(&pause);
        
        Cpu {
            memory : temp_memory,
            v : [0; 16],
            i : 0,
            pc : 0x200, // 0x0 to 0x199 is reserved for the interpreter (fonts...)
            timers : timers,
            pause : pause,
            stack : Vec::new(),
            graphics_subsystem : Box::new(Graphics::new(&sdl_context, config, ttf_context)),
            keypad_subsystem : Box::new(Keypad::new(&sdl_context, pause_inner, freq_period)),
            wants_to_quit : false,
            instr_log : Vec::new(),
            config : config
        }
    }
    
    /// Executes a cycle
    pub fn cycle(&mut self)  {
        if ! *self.pause.borrow() {
            // Fetch Opcode
            // Shift the first part of the instr to the left and merge the second part on it
            let instr : u16 = (self.memory[self.pc] as u16) << 8 | (self.memory[self.pc + 1] as u16);

            // Log it
            self.instr_log.insert(0, instr);
            self.instr_log.truncate(12); // Keep a reasonable log size

            // Decode and execute 
            self.execute_instr(instr);
        }
    }
    
    pub fn poll_keypad(&mut self) -> bool {
        self.keypad_subsystem.poll_keyboard()
    }

    pub fn finished(&self) -> bool {
        self.wants_to_quit
    }


    fn execute_instr(&mut self, instr : u16) {
        // Divide the 16-bit instr into 4 groups of 4 bits (represented as an u8)
        let instr_nibbles = (
            //                  AAAA BBBB CCCC DDDD
            // BITWISE_AND      1111 0000 0000 0000
            //              =   AAAA 0000 0000 0000
            // >> 12        =   0000 0000 0000 AAAA
            // as u8        =   0000 AAAA
            (instr & 0xF000) >> 12 as u8,
            // Same with the rest
            (instr & 0x0F00) >> 8 as u8,
            (instr & 0x00F0) >> 4 as u8,
            (instr & 0x000F) as u8,
        );

        // Address part of the instr
        let nnn = (instr & 0x0FFF) as usize;
        // 8-bit constant
        let nn = (instr & 0x00FF) as u8;
        // 4-bit constant
        let n = instr_nibbles.3 as usize;
        // 4-bit v
        let x = instr_nibbles.1 as usize;
        let y = instr_nibbles.2 as usize;

        let pc_change = match instr_nibbles { 
            // ONNN
            (0x00, 0x00, 0x0e, 0x00) => self.op_00e0(),
            (0x00, 0x00, 0x0e, 0x0e) => self.op_00ee(),
            (0x01, _, _, _) => self.op_1nnn(nnn),
            (0x02, _, _, _) => self.op_2nnn(nnn),
            (0x03, _, _, _) => self.op_3xkk(x, nn),
            (0x04, _, _, _) => self.op_4xkk(x, nn),
            (0x05, _, _, 0x00) => self.op_5xy0(x, y),
            (0x06, _, _, _) => self.op_6xnn(x, nn),
            (0x07, _, _, _) => self.op_7xnn(x, nn),
            (0x08, _, _, 0x00) => self.op_8xy0(x, y),
            (0x08, _, _, 0x01) => self.op_8xy1(x, y),
            (0x08, _, _, 0x02) => self.op_8xy2(x, y),
            (0x08, _, _, 0x03) => self.op_8xy3(x, y),
            (0x08, _, _, 0x04) => self.op_8xy4(x, y),
            (0x08, _, _, 0x05) => self.op_8xy5(x, y),
            (0x08, _, _, 0x06) => self.op_8x06(x),
            (0x08, _, _, 0x07) => self.op_8xy7(x, y),
            (0x08, _, _, 0x0e) => self.op_8xye(x),
            (0x09, _, _, 0x00) => self.op_9xy0(x, y),
            (0x0a, _, _, _) => self.op_annn(nnn),
            (0x0b, _, _, _) => self.op_bnnn(nnn),
            (0x0c, _, _, _) => self.op_cxnn(x, nn),
            (0x0d, _, _, _) => self.op_dxyn(x, y, n),
            (0x0e, _, 0x09, 0x0e) => self.op_ex9e(x),
            (0x0e, _, 0x0a, 0x01) => self.op_exa1(x),
            (0x0f, _, 0x00, 0x07) => self.op_fx07(x),
            (0x0f, _, 0x00, 0x0a) => self.op_fx0a(x),
            (0x0f, _, 0x01, 0x05) => self.op_fx15(x),
            (0x0f, _, 0x01, 0x08) => self.op_fx18(x),
            (0x0f, _, 0x01, 0x0e) => self.op_fx1e(x),
            (0x0f, _, 0x02, 0x09) => self.op_fx29(x),
            (0x0f, _, 0x03, 0x03) => self.op_fx33(x),
            (0x0f, _, 0x05, 0x05) => self.op_fx55(x),
            (0x0f, _, 0x06, 0x05) => self.op_fx65(x),
            _ => NextPCValue::Next,
        };
            
        
        
        match pc_change {
            NextPCValue::Next => self.pc += 2, // PC addresses 16 bits, so we need to advance 2 bytes
            NextPCValue::Skip => self.pc += 4, // Same, skipping the next instruction
            NextPCValue::Jump(new) => self.pc = new,
        }
    }

    /// Clears the screen. 
    fn op_00e0(&mut self) -> NextPCValue {
        self.graphics_subsystem.clear_screen();

        NextPCValue::Next
    }

    /// Returns from a subroutine. 
    fn op_00ee(&mut self) -> NextPCValue {
        NextPCValue::Jump(self.stack.pop().unwrap()) // We need to panic if we try to jump back to a non-existent routine
    }

    /// Jumps to address NNN.
    fn op_1nnn(&mut self, nnn : usize) -> NextPCValue {
        NextPCValue::Jump(nnn)
    }

    /// Calls subroutine at NNN. 
    fn op_2nnn(&mut self, nnn: usize) -> NextPCValue {
        self.stack.push(self.pc+2); // Store the next PC value

        NextPCValue::Jump(nnn)
    }
    
    /// Skips the next instruction if VX equals NN. 
    /// (Usually the next instruction is a jump to skip a code block); 
    fn op_3xkk(&mut self, x: usize, nn: u8) -> NextPCValue {
        if self.v[x] == nn {
            return NextPCValue::Skip;
        }

        NextPCValue::Next
    }

    /// Skips the next instruction if VX does not equal NN. 
    /// (Usually the next instruction is a jump to skip a code block); 
    fn op_4xkk(&mut self, x: usize, nn: u8) -> NextPCValue {
        if self.v[x] != nn {
            return NextPCValue::Skip;
        }

        NextPCValue::Next
    }

    /// Skips the next instruction if VX equals VY. 
    /// (Usually the next instruction is a jump to skip a code block); 
    fn op_5xy0(&mut self, x: usize, y: usize) -> NextPCValue {
        if self.v[x] == self.v[y] {
            return NextPCValue::Skip;
        }
        
        NextPCValue::Next
    }

    /// Sets VX to NN. 
    fn op_6xnn(&mut self, x: usize, nn: u8) -> NextPCValue {
        self.v[x] = nn;

        NextPCValue::Next
    }

    /// Adds NN to VX. (Carry flag (VF) is not changed); 
    fn op_7xnn(&mut self, x: usize, nn: u8) -> NextPCValue {
        // https://doc.rust-lang.org/std/primitive.u8.html#method.overflowing_add
        // Wraps around, and we don't care whether there's overflow or not
        self.v[x] = self.v[x].wrapping_add(nn); 
        
        NextPCValue::Next
    }

    /// Sets VX to the value of VY. 
    fn op_8xy0(&mut self, x: usize, y: usize) -> NextPCValue {
        self.v[x] = self.v[y];
        
        NextPCValue::Next
    }

    /// Sets VX to (VX or VY). (Bitwise OR operation); 
    fn op_8xy1(&mut self, x: usize, y: usize) -> NextPCValue {
        self.v[x] |= self.v[y];
        
        NextPCValue::Next
    }

    /// Sets VX to VX and VY. (Bitwise AND operation); 
    fn op_8xy2(&mut self, x: usize, y: usize) -> NextPCValue {
        self.v[x] &= self.v[y];

        NextPCValue::Next
    }

    /// Sets VX to VX xor VY. 
    fn op_8xy3(&mut self, x: usize, y: usize) -> NextPCValue {
        self.v[x] ^= self.v[y];
        
        NextPCValue::Next
    }

    /// Adds VY to VX. VF is set to 1 when there's a carry, and to 0 when there is not. 
    fn op_8xy4(&mut self, x: usize, y: usize) -> NextPCValue {
        // https://doc.rust-lang.org/std/primitive.u8.html#method.overflowing_add
        // Wraps around and returns true if an overflow occurs
        let (result, overflow) = self.v[x].overflowing_add(self.v[y]);

        self.v[x] = result;
        self.v[0x0f] = if overflow { 1 } else { 0 };
        
        NextPCValue::Next
    }

    /// VY is subtracted from VX. VF is set to 0 when there's a borrow, and 1 when there is not. 
    fn op_8xy5(&mut self, x: usize, y: usize) -> NextPCValue {
        // https://doc.rust-lang.org/std/primitive.u8.html#method.overflowing_sub
        // Wraps around and returns true if an overflow occurs
        let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);

        self.v[x] = result;
        self.v[0x0f] = if overflow { 0 } else { 1 };

        NextPCValue::Next
    }

    /// Stores the least significant bit of VX in VF and then shifts VX to the right by 1
    fn op_8x06(&mut self, x: usize) -> NextPCValue {
        self.v[0x0f] = self.v[x] & 0b00000001;
        self.v[x] >>= 1;
        
        NextPCValue::Next
    }

    /// Sets VX to VY minus VX. VF is set to 0 when there's a borrow, and 1 when there is not. 
    fn op_8xy7(&mut self, x: usize, y: usize) -> NextPCValue {
        // https://doc.rust-lang.org/std/primitive.u8.html#method.overflowing_sub
        // Wraps around and returns true if an overflow occurs
        let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
        
        self.v[x] = result;
        self.v[0x0f] = if overflow { 0 } else { 1 };

        NextPCValue::Next
    }

    // Stores the most significant bit of VX in VF and then shifts VX to the left by 1
    fn op_8xye(&mut self, x: usize) -> NextPCValue {
        self.v[0x0f] = (self.v[x] & 0b10000000) >> 7;
        self.v[x] <<= 1;
        
        NextPCValue::Next
    }

    /// Skips the next instruction if VX does not equal VY. (Usually the next instruction is a jump to skip a code block)
    fn op_9xy0(&mut self, x: usize, y: usize) -> NextPCValue {
        if self.v[x] != self.v[y] {
            return NextPCValue::Skip;
        }

        NextPCValue::Next
    }

    /// Sets I to the address NNN
    fn op_annn(&mut self, nnn: usize) -> NextPCValue {
        self.i = nnn;
        
        NextPCValue::Next
    }

    /// Jumps to the address NNN plus V0. 
    fn op_bnnn(&mut self, nnn: usize) -> NextPCValue {
        NextPCValue::Jump((self.v[0] as usize) + nnn)
    }

    /// Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN. 
    fn op_cxnn(&mut self, x: usize, nn: u8) -> NextPCValue {
        let mut rng = rand::thread_rng();
        self.v[x] = rng.gen_range(0..255 as u8) & nn;
        
        NextPCValue::Next
    }

    /// Draws a sprite at coordinate (VX, VY) that has a width of 8 pixels and a height of N pixels. 
    /// Each row of 8 pixels is read as bit-coded starting from memory location I; (address register)
    /// I value does not change after the execution of this instruction. 
    /// As described above, VF is set to 1 if any screen pixels are flipped from set to unset 
    /// when the sprite is drawn, and to 0 if that does not happen 
    fn op_dxyn(&mut self, x: usize, y: usize, n: usize) -> NextPCValue {
        // https://tobiasvl.github.io/blog/write-a-chip-8-emulator/#dxyn-display
        // The starting coordinates and the drawing itself are wrapped depending on the config option
        self.v[0x0f] = 0;
    
        for height in 0..n {
            let y_coord;

            if ! self.config.wrapping_enabled() {
                y_coord = self.v[y] as usize + height; 
            } else {
                y_coord = (self.v[y] as usize + height) % 32;
            }

            for width in 0..8 {
                let x_coord; 

                if ! self.config.wrapping_enabled() {
                    x_coord = self.v[x] as usize + width;
                } else {
                    x_coord = (self.v[x] as usize + width) % 64;
                }

                // gets the corresponding column value of the row by shifting, starting from the MSB
                let color = (self.memory[self.i + height] >> (7 - width)) & 0b00000001;

                self.v[0x0f] |= self.graphics_subsystem.set_pos(x_coord, y_coord, color);
            }
        }
        
        self.graphics_subsystem.draw(&self.v, &self.stack, &self.instr_log);

        NextPCValue::Next
    }

    /// Skips the next instruction if the key stored in VX is pressed. 
    /// (Usually the next instruction is a jump to skip a code block); 
    fn op_ex9e(&mut self, x: usize) -> NextPCValue {
        if self.keypad_subsystem.is_pressed(self.v[x] as usize) {
            return NextPCValue::Skip;
        }
        
        NextPCValue::Next        
    }

    /// Skips the next instruction if the key stored in VX is not pressed. 
    /// (Usually the next instruction is a jump to skip a code block); 
    fn op_exa1(&mut self, x: usize) -> NextPCValue {
        if ! self.keypad_subsystem.is_pressed(self.v[x] as usize) {
            return NextPCValue::Skip;
        }

        NextPCValue::Next  
    }

    
    /// Sets VX to the value of the delay timer. 
    fn op_fx07(&mut self, x: usize) -> NextPCValue {
        let (delay_timer, _) = *self.timers.lock().unwrap();

        self.v[x] = delay_timer;
        
        NextPCValue::Next
    }


    /// A key press is awaited, and then stored in VX. 
    /// Blocking Operation. (All instructions are halted until next key event)
    fn op_fx0a(&mut self, x: usize) -> NextPCValue {
        for i in self.keypad_subsystem.iter() {
            if *i {
                self.v[x] = *i as u8;
                return NextPCValue::Next;
            }
        }

        NextPCValue::Jump(self.pc) // "jump" to the same instruction again
    }


    /// Sets the delay timer to VX
    fn op_fx15(&mut self, x: usize) -> NextPCValue {
        if let Ok(mut timers) = self.timers.lock() {
            let (_ , sound_timer ) = *timers;
            
            *timers = (self.v[x], sound_timer);
        }
    
        NextPCValue::Next
    }

    /// Sets the sound timer to VX
    fn op_fx18(&mut self, x: usize) -> NextPCValue {
        if let Ok(mut timers) = self.timers.lock() {
            let (delay_timer , _ ) = *timers;
            
            *timers = (delay_timer, self.v[x]);
        }
        
        NextPCValue::Next
    }

    /// Adds VX to I. VF is not affected
    fn op_fx1e(&mut self, x: usize) -> NextPCValue {
        self.i += self.v[x] as usize;
        
        NextPCValue::Next
    }

    /// Sets I to the location of the sprite for the character in VX
    /// Characters 0-F (in hexadecimal) are represented by a 4x5 font. 
    fn op_fx29(&mut self, x: usize) -> NextPCValue {
        // Fonts are pre-allocated starting from 0x0, and each one is 5 bytes long        
        self.i = (self.v[x] as usize) * 5;

        NextPCValue::Next
    }

    /// Stores the binary-coded decimal representation of VX, with the most significant of three digits at the address in I, 
    /// the middle digit at I plus 1, and the least significant digit at I plus 2. 
    ///
    /// (In other words, take the decimal representation of VX, place the hundreds digit in memory at location in I, 
    /// the tens digit at location I+1, and the ones digit at location I+2.); 
    fn op_fx33(&mut self, x: usize) -> NextPCValue {
        self.memory[self.i] = self.v[x] / 100; // hundreds digit

        self.memory[self.i + 1] = (self.v[x] % 100) / 10; // tens digit

        self.memory[self.i + 2] = self.v[x] % 10; // ones digit
        
        NextPCValue::Next
    }

    /// Stores V0 to VX (including VX) in memory starting at address I
    /// The offset from I is increased by 1 for each value written, but I itself is left unmodified
    fn op_fx55(&mut self, x: usize) -> NextPCValue {
        for i in 0..=x {
            self.memory[self.i + i] = self.v[i];
        }

        NextPCValue::Next
    }

    /// Fills V0 to VX (including VX) with values from memory starting at address I. 
    /// The offset from I is increased by 1 for each value written, but I itself is left unmodified.
    fn op_fx65(&mut self, x: usize) -> NextPCValue {
        for i in 0..=x {
            self.v[i] = self.memory[self.i + i];
        }

        NextPCValue::Next
    }

    fn load_fonts(memory : &mut [u8; 4096]) {
        let mut i = 0;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x20; i+=1;
        memory[i] = 0x60; i+=1;
        memory[i] = 0x20; i+=1;
        memory[i] = 0x20; i+=1;
        memory[i] = 0x70; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0x20; i+=1;
        memory[i] = 0x40; i+=1;
        memory[i] = 0x40; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x10; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xE0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xE0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xE0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xE0; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0x90; i+=1;
        memory[i] = 0xE0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0xF0; i+=1;
        memory[i] = 0x80; i+=1;
        memory[i] = 0x80;
    }

    fn load_rom(path : &str, memory : &mut [u8; 4096]) {
        let mut file = File::open(path).unwrap();
        
        // Insert the ROM contents, starting from 0x200
        file.read(&mut memory[0x200..]).unwrap();
    }
}

// This is private
mod cpu; // Promise chip8 is defined either in `./cpu.rs` or `./cpu/mod.rs`,
mod graphics; // etc.
mod keypad;
mod sound;
mod timer;

// Re-export cpu's functions and structs
pub use cpu::*; // Bring all symbols in scope, which we promise the `cpu` module exports.
pub use graphics::*; // etc.
pub use keypad::*;
pub use sound::*;
pub use timer::*;

// https://fasterthanli.me/articles/rust-modules-vs-files

// Scope structure:
//`cpu` module (YOU ARE HERE)
//    `Cpu` struct (public)
//    ... (public)  
//
//    `cpu` module (private)
//          `Cpu` struct (public)
//          ... (public)  

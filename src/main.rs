use sdl2;

//#[path = "cpu/cpu.rs"] // Another way to do it
mod chip8;
mod config;

use std::{thread, time};
//use std::time::SystemTime;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::mpsc::{self};
use structopt::StructOpt;

fn main() {
    // https://jackson-s.me/2019/07/13/Chip-8-Instruction-Scheduling-and-Frequency.html
    // we run the main loop at 550hz (~1.82ms), and the timers at 60Hz
    
    let freq_period : Rc<RefCell<u64>> = Rc::new(RefCell::new(1820000)); // Shared with they keypad, inside the cpu
    let config = config::Config::from_args();

    // SDL2
    let sdl_context = sdl2::init().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();

    // Timers and pause shared variables
    let timers : Arc<Mutex<(u8, u8)>> = Arc::new(Mutex::new((0,0)));
    let pause : Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    // Cpu
    let mut cpu = chip8::Cpu::new(&sdl_context, &config, Arc::clone(&timers), Rc::clone(&pause), Rc::clone(&freq_period), ttf_context);
    let mut wants_to_quit = false;
    
    // Timer loop and beep flag
    let (tx, rx) = mpsc::channel();

    let must_beep = Arc::new(Mutex::new(false));

    let must_beep_inner = Arc::clone(&must_beep);
    let handler = thread::spawn(move || {
        let mut timer_subsystem = chip8::Timer::new(Arc::clone(&timers), rx, must_beep_inner);
        timer_subsystem.run();
    });

    // Sound subsystem
    let sound_subsystem = chip8::Sound::new(&sdl_context);

    while ! (cpu.finished() || wants_to_quit) {
        wants_to_quit = cpu.poll_keypad();
        
        cpu.cycle();    
        
        if * must_beep.lock().unwrap() {
            sound_subsystem.beep();
        } else {
            sound_subsystem.stop_beep();
        }

        thread::sleep(time::Duration::from_nanos(*freq_period.borrow()));
    }

    let _ = tx.send(()); // Tell the timer subsystem to stop
    handler.join().unwrap();
    println!("Terminating VM...");
}

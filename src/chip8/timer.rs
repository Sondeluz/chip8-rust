use std::{thread, time};
use std::sync::Mutex;
use std::sync::Arc;
use std::sync::mpsc::{TryRecvError};

// a 60hz timer supposed to run in a thread, which updates the CPU timers
pub struct Timer {
    timers : Arc<Mutex<(u8, u8)>>, // Shared timers between the CPU and this timer thread
    rx : std::sync::mpsc::Receiver<()>, // Receiving end of the channel between the main thread and this timer thread
    must_beep : Arc<Mutex<bool>>    // We cannot bring the audio subsystem here due to sdl2
                                    // being limited to one thread, so as a workaround we set
                                    // off a flag
}

impl Timer {
    pub fn new(timers : Arc<Mutex<(u8, u8)>>, rx : std::sync::mpsc::Receiver<()>, must_beep : Arc<Mutex<bool>>) -> Timer {
        Timer {
            timers : timers,
            rx : rx,
            must_beep : must_beep
        }
    }

    /// Intended to be run as a thread, updates the timers emulating ~60hz cycles
    pub fn run(&mut self) {
        loop {
            // Check if we should end
            match self.rx.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    println!("Terminating timer subsystem...");
                    break;
                }

                Err(TryRecvError::Empty) => {}
            }
    

            if let Ok(mut timers) = self.timers.lock() {
                let (mut delay_timer, mut sound_timer) = *timers;

                if delay_timer > 0 {
                    delay_timer -= 1;
                }
                
                if sound_timer > 0 {
                    sound_timer -= 1;
                    // The system should beep once the sound timer gets to 0
                    if sound_timer != 0 {
                        * self.must_beep.lock().unwrap() = true;
                    } else {
                        * self.must_beep.lock().unwrap() = false;
                    }
                }

                *timers = (delay_timer, sound_timer);
            }
            
            thread::sleep(time::Duration::from_nanos(16666667)); // It should tick at 60hz, this is...approximate
        }
    }
}

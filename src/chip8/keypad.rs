use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::rc::Rc;
use std::cell::RefCell;

pub const EXIT_KEY_VALUE : usize = 0xffa;
const EXIT_KEYCODE : Keycode = Keycode::Escape;
pub const PAUSE_KEY_VALUE : usize = 0xffb;
const PAUSE_KEYCODE : Keycode = Keycode::Space;
pub const FREQ_DOWN_KEY_VALUE : usize = 0xffc;
const FREQ_DOWN_KEYCODE : Keycode = Keycode::Down;
pub const FREQ_UP_KEY_VALUE : usize = 0xffd;
const FREQ_UP_KEYCODE : Keycode = Keycode::Up;

pub struct Keypad {
    keypad : [bool; 16],
    event_pump : sdl2::EventPump,
    pause : Rc<RefCell<bool>>, // shared pause flag, read by the cpu
    freq_period : Rc<RefCell<u64>>
}

impl Keypad {
    pub fn new(sdl_context : &sdl2::Sdl, pause : Rc<RefCell<bool>>, freq_period : Rc<RefCell<u64>>) -> Keypad {
        Keypad {
            keypad : [false; 16],
            event_pump : sdl_context.event_pump().unwrap(), // get and handle the event pump from the context
            pause : pause,
            freq_period : freq_period
        }
    }

    // Return an iterator over the keypad
    pub fn iter(&self) -> std::slice::Iter<bool> {
        self.keypad.iter()
    }

    /// Checks if the key is pressed
    pub fn is_pressed(&mut self, key : usize) -> bool {
        if (0..=0xF).contains(&key) {
            return self.keypad[key]; 
        }

        // If the key was out of bounds...
        false
    }

    /// Consumes all SDL events and updates the keypad. Returns true if the user
    /// wants to quit, false otherwise.
    pub fn poll_keyboard(&mut self) -> bool {
        let mut wants_to_quit = false;

        // Consumes all pending events and checks if one of them is quitting (pressing (x) in the window...)
        for event in self.event_pump.poll_iter() { 
            if let Event::Quit { .. } = event {
                wants_to_quit = true;
            };
        } 

        let keys: Vec<Keycode> = self.event_pump
            .keyboard_state() // Get a snapshot of the current keyboard state
            .pressed_scancodes() // With the pressed scancodes
            .filter_map(Keycode::from_scancode) // Turning them into keycodes
            .collect(); // And into a Vec

        self.clear_keypad();

        for key in keys {
            // https://tobiasvl.github.io/assets/images/cosmac-vip-keypad.png
            let index = match key {
                Keycode::Num1 => Some(0x1),
                Keycode::Num2 => Some(0x2),
                Keycode::Num3 => Some(0x3),
                Keycode::Num4 => Some(0xc),
                Keycode::Q => Some(0x4),
                Keycode::W => Some(0x5),
                Keycode::E => Some(0x6),
                Keycode::R => Some(0xd),
                Keycode::A => Some(0x7),
                Keycode::S => Some(0x8),
                Keycode::D => Some(0x9),
                Keycode::F => Some(0xe),
                Keycode::Z => Some(0xa),
                Keycode::X => Some(0x0),
                Keycode::C => Some(0xb),
                Keycode::V => Some(0xf),
                EXIT_KEYCODE => Some(EXIT_KEY_VALUE), // Exit key
                PAUSE_KEYCODE => Some(PAUSE_KEY_VALUE),
                FREQ_DOWN_KEYCODE => Some(FREQ_DOWN_KEY_VALUE), // Exit key
                FREQ_UP_KEYCODE => Some(FREQ_UP_KEY_VALUE),
                _ => None,
            };

            if let Some(i) = index {
                match i {
                    EXIT_KEY_VALUE => wants_to_quit = true, 
                    PAUSE_KEY_VALUE => {    
                        let pause = *self.pause.borrow();
                        *self.pause.borrow_mut() = ! pause;
                    },
                    FREQ_DOWN_KEY_VALUE => {
                            let freq = *self.freq_period.borrow();
                            *self.freq_period.borrow_mut() = freq.saturating_add(1000);
                        },
                    FREQ_UP_KEY_VALUE => {
                            let freq = *self.freq_period.borrow();
                            *self.freq_period.borrow_mut() = freq.saturating_sub(1000);
                        }
                    i => self.keypad[i] = true
                }
            }
        }

        wants_to_quit
    }

    /// Self-explanatory
    fn clear_keypad(&mut self) {
        for key in self.keypad.iter_mut() {
            *key = false;
        }
    }
}

use sdl2;
use sdl2::pixels;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::render::TextureQuery;
use sdl2::pixels::Color;

use crate::config;
// Pretty much based on https://github.com/starrhorne/chip8-rust/blob/master/src/drivers/display_driver.rs,
// modified to bring the screen matrix here, and also draw information about the CPU state

// Since the chip8 screen is 64x32, we scale it
const SCALE_FACTOR: u32 = 15;

// handle the annoying Rect i32
// https://github.com/Rust-SDL2/rust-sdl2/blob/master/examples/ttf-demo.rs
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

pub struct Graphics<'a> {
    screen : [[u8; 64]; 32], // graphics matrix
    canvas: Canvas<Window>,
    ttf_context : sdl2::ttf::Sdl2TtfContext,
    config : &'a config::Config,
    texture_creator : sdl2::render::TextureCreator<sdl2::video::WindowContext>,
}

impl Graphics<'_> {
    pub fn new<'a>(sdl_context : &'a sdl2::Sdl, config : &'a config::Config, ttf_context : sdl2::ttf::Sdl2TtfContext) -> Graphics<'a> {
        // Initialization
        let video_subsys = sdl_context.video().unwrap();
        let window = video_subsys
            // only widths up to 63 * SCALE_FACTOR are used by the game itself, the rest are for the VM to draw information on
            .window("CHIP-8 VM", 128 * SCALE_FACTOR, 32 * SCALE_FACTOR) 
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        let texture_creator = canvas.texture_creator();

        Graphics {
            screen : [[0; 64]; 32],
            canvas: canvas,
            ttf_context : ttf_context,
            config : config,
            texture_creator : texture_creator,
        }
    }

    pub fn clear_screen(&mut self) {
        for row in self.screen.iter_mut() {
            for col in row.iter_mut() {
                *col = 0;
            }
        }
    }

    /// If the coordinates are correct, XORs the value at (x,y).
    /// Returns 1 if the screen pixel has changed from set to unset, otherwise 0
    pub fn set_pos(&mut self, x : usize, y : usize, val : u8) -> u8 {
        let mut changed = 0;
        
        if ! self.config.wrapping_enabled() {
            if (0..64).contains(&x) && (0..32).contains(&y) {
                changed = self.screen[y][x]; // y is indexed first, it's a 2d array!
                // The value is XOR'd into the screen
                self.screen[y][x] ^= val; 

                // And the changed flag is activated if the pixel is    
                // unset, which only happens if both values were 1 due to
                // the XOR operation
                changed &= val;
            }
        } else { // We mod the coordinates to the maximum values and thus wrap them
            changed = self.screen[y % 32][x % 64]; // y is indexed first, it's a 2d array!
            // The value is XOR'd into the screen
            self.screen[y % 32][x % 64] ^= val; 

            // And the changed flag is activated if the pixel is    
            // unset, which only happens if both values were 1 due to
            // the XOR operation
            changed &= val;
        }

        

        changed
    }

    pub fn draw(&mut self, v : &[u8; 16], stack : &Vec<usize>, instr_log : &Vec<u16>) {
        // Load the font
        let mut font = self.ttf_context.load_font(self.config.font_path(), 128).unwrap();
        font.set_style(sdl2::ttf::FontStyle::BOLD);

        self.canvas.clear();

        // CPU registers
        let surface = font
            .render(&format!("Register contents:    \
                                v0:   {:#06x}   v1:   {:#06x}   \
                                v2:   {:#06x}   v3:   {:#06x}   \
                                v4:   {:#06x}   v5:   {:#06x}   \
                                v6:   {:#06x}   v7:   {:#06x}   \
                                v8:   {:#06x}   v9:   {:#06x}   \
                                v10:   {:#06x}   v11:   {:#06x}   \
                                v12:   {:#06x}   v13:   {:#06x}   \
                                v14:   {:#06x}   v15:   {:#06x}   ", 
                                v[0], v[1], v[2], v[3], v[4], v[5], 
                                v[6], v[7], v[8], v[9], v[10], v[11], 
                                v[12], v[13], v[14], v[15]))
            .blended_wrapped(Color::RGBA(194, 57, 56, 0), 1200)
            .map_err(|e| e.to_string()).unwrap();
        
        let texture_cpu = self.texture_creator.create_texture_from_surface(&surface).unwrap();
        let rect_cpu = self.get_rect_cpu_registers(&texture_cpu);

        // Stack
        let mut stack_arr : [usize; 12] = [0; 12]; // The default/original stack size was 12
        let mut i = 0;
        for elem in stack.iter().rev() {
            stack_arr[i] = *elem;
            i += 1;
        }

        let surface = font
            .render(&format!("Stack:    {:#06x}    {:#06x}    {:#06x}    \
                                {:#06x}    {:#06x}    {:#06x}    {:#06x}    \
                                {:#06x}    {:#06x}    {:#06x}    {:#06x}    {:#06x}", 
                                stack_arr[0], stack_arr[1], stack_arr[2], stack_arr[3], 
                                stack_arr[4], stack_arr[5], stack_arr[6], stack_arr[7], 
                                stack_arr[8], stack_arr[9], stack_arr[10], stack_arr[11]))
            .blended_wrapped(Color::RGBA(194, 57, 56, 0), 1200)
            .map_err(|e| e.to_string()).unwrap();
        
        let texture_stack = self.texture_creator.create_texture_from_surface(&surface).unwrap();
        let rect_stack = self.get_rect_stack(&texture_stack);

        // Instructions
        let mut instr_log_arr : [u16; 12] = [0;12];
        let mut i = 0;
        for instr in instr_log.iter() {
            instr_log_arr[i] = *instr;
            i += 1;
        }
    
        let surface = font
            .render(&format!("Instruction history:    {:#06x}    {:#06x}    {:#06x}    \
                                {:#06x}    {:#06x}    {:#06x}    {:#06x}    {:#06x}    \
                                {:#06x}    {:#06x}    {:#06x}    {:#06x}", 
                                instr_log_arr[0], instr_log_arr[1], instr_log_arr[2], 
                                instr_log_arr[3], instr_log_arr[4], instr_log_arr[5], 
                                instr_log_arr[6], instr_log_arr[7], instr_log_arr[8], 
                                instr_log_arr[9], instr_log_arr[10], instr_log_arr[11]))
            .blended_wrapped(Color::RGBA(194, 57, 56, 0), 1200)
            .map_err(|e| e.to_string()).unwrap();
        
        let texture_instr = self.texture_creator.create_texture_from_surface(&surface).unwrap();
        let rect_instr = self.get_rect_instr(&texture_instr);

        self.canvas.copy(&texture_cpu, None, Some(rect_cpu)).unwrap();
        self.canvas.copy(&texture_stack, None, Some(rect_stack)).unwrap();
        self.canvas.copy(&texture_instr, None, Some(rect_instr)).unwrap();

        for (y, row) in self.screen.iter().enumerate() { // Iterate through each row
            for (x, &col_value) in row.iter().enumerate() { // Iterator through each column
                // Scale the coords
                let x = (x as u32) * SCALE_FACTOR;
                let y = (y as u32) * SCALE_FACTOR;
                
                // if it has a non-zero value, the pixel is active
                if col_value == 0 {
                    self.canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
                } else {    
                    self.canvas.set_draw_color(pixels::Color::RGB(198, 43, 248)); // I like purple
                }
                
                // Draws the pixel as a rectangle
                self.canvas.fill_rect(Rect::new(x as i32, y as i32, SCALE_FACTOR, SCALE_FACTOR)).unwrap();
            }
        }
        self.canvas.present();
    }

    // All functions below are based on the SDL2 ttf demo at https://github.com/Rust-SDL2/rust-sdl2/blob/master/examples/ttf-demo.rs

    fn get_rect_cpu_registers(&self, texture : &sdl2::render::Texture) -> Rect {
        let TextureQuery { width, height, .. } = texture.query();
        // If the example text is too big for the screen, downscale it (and position it irregardless)
        let padding = 0;
        self.get_rect_aligned_left(
            width,
            height,
            (128 - padding) * SCALE_FACTOR,
            (32 - padding) * SCALE_FACTOR,
        )
    }

    fn get_rect_stack(&self, texture : &sdl2::render::Texture) -> Rect {
        let TextureQuery { width, height, .. } = texture.query();
        // If the example text is too big for the screen, downscale it (and position it irregardless)
        let padding = 0;
        self.get_rect_aligned_right(
            width,
            height,
            (128 - padding) * SCALE_FACTOR,
            (32 - padding) * SCALE_FACTOR,
        )
    }

    fn get_rect_instr(&self, texture : &sdl2::render::Texture) -> Rect {
        let TextureQuery { width, height, .. } = texture.query();
        // If the example text is too big for the screen, downscale it (and position it irregardless)
        let padding = 0;
        self.get_rect_aligned_center(
            width,
            height,
            (128 - padding) * SCALE_FACTOR,
            (32 - padding) * SCALE_FACTOR,
        )
    }

    // Scale fonts to a reasonable size when they're too big (though they might look less smooth)
    fn get_rect_aligned_left(&self, rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
        let wr = rect_width as f32 / cons_width as f32;
        let hr = rect_height as f32 / cons_height as f32;

        let (w, h) = if wr > 1f32 || hr > 1f32 {
            if wr > hr {
                let h = (rect_height as f32 / wr) as i32;
                (cons_width as i32, h)
            } else {
                let w = (rect_width as f32 / hr) as i32;
                (w, cons_height as i32)
            }
        } else {
            (rect_width as i32, rect_height as i32)
        };

        rect!(65*SCALE_FACTOR, 0, w, h)
    }

    // Scale fonts to a reasonable size when they're too big (though they might look less smooth)
    fn get_rect_aligned_right(&self, rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
        let wr = rect_width as f32 / cons_width as f32;
        let hr = rect_height as f32 / cons_height as f32;

        let (w, h) = if wr > 1f32 || hr > 1f32 {
            if wr > hr {
                let h = (rect_height as f32 / wr) as i32;
                (cons_width as i32, h)
            } else {
                let w = (rect_width as f32 / hr) as i32;
                (w, cons_height as i32)
            }
        } else {
            (rect_width as i32, rect_height as i32)
        };

        let cx = (128*SCALE_FACTOR as i32 - w) / 2 + 64 * SCALE_FACTOR as i32;
        rect!(cx, 0, w, h)
    }

    // Scale fonts to a reasonable size when they're too big (though they might look less smooth)
    fn get_rect_aligned_center(&self, rect_width: u32, rect_height: u32, cons_width: u32, cons_height: u32) -> Rect {
        let wr = rect_width as f32 / cons_width as f32;
        let hr = rect_height as f32 / cons_height as f32;

        let (w, h) = if wr > 1f32 || hr > 1f32 {
            if wr > hr {
                let h = (rect_height as f32 / wr) as i32;
                (cons_width as i32, h)
            } else {
                let w = (rect_width as f32 / hr) as i32;
                (w, cons_height as i32)
            }
        } else {
            (rect_width as i32, rect_height as i32)
        };

        let cx = (128*SCALE_FACTOR as i32 - w) / 2 + 32 * SCALE_FACTOR as i32;

        rect!(cx, 0, w, h)
    }
    
}

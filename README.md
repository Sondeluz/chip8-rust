![Playing BLITZ with the "debugger" on the right](https://github.com/Sondeluz/chip8-rust/blob/master/demo.gif)

## Introduction

This is a fork of [starrhorne's](https://github.com/starrhorne/chip8-rust) amazing CHIP-8 emulator, written in Rust.
I wanted to play with Rust's low-level stuff and work on a small emulator, and this was a great choice. 
This is pretty much based on starrhorne's project, hence the fork. 
Some parts of it are the same source files, self-commented, with a few tweaks here and there which don't really change the overall behavior.

I added a few functions such as being able to see the CPU registers, stack contents and a small instruction history,
which requires providing a valid .ttf file for displaying text. I included one (Terminus TTF) in the project.

I also moved around some structural parts, such as the timers which now reside in a separate thread running at 60Hz.

Finally, it is also possible to:

- Pause the emulation by pressing the spacebar.
- Increase the game's frequency by pressing the Up arrow. (Doesn't affect the timers)
- Decrease the game's frequency by pressing the Up arrow. (Doesn't affect the timers)
- Toggle sprite wrapping on/off, as some games require wrapping, and others not (via arguments).

## Resources

Please refer to [starrhorne's](https://github.com/starrhorne/chip8-rust) resources tips.

## Requirements

Apart from needing to have sdl2 libraries installed, it is also now required to have sdl2-ttf libraries too.

## Usage

Clone this repository, then run the executable, arguments can be found below:

```
USAGE:
    chip-8-vm [FLAGS] [OPTIONS] <rom-path>

FLAGS:
    -h, --help                Prints help information
    -V, --version             Prints version information
    -w, --wrapping_enabled    Enable sprite wrapping on the borders of the screen (needed by some games, such as BLITZ)

OPTIONS:
    -f, --font_path <font_path>    Path to the font needed to display information [default: font.ttf]

```

You can find public-domain games [here](https://www.zophar.net/pdroms/chip8/chip-8-games-pack.html). 

While playing, you can:
- Pause the emulation by pressing the spacebar.
- Increase the game's frequency by pressing the Up arrow. 
- Decrease the game's frequency by pressing the Up arrow.
- Exit the application by pressing Escape (or closing the window)

## Credits

[starrhorne's project](https://github.com/starrhorne/chip8-rust).

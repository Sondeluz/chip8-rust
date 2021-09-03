use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "A CHIP-8 VM implementation", about = "CHIP-8 VM. Pass `-h` to see all optional flags")]
pub struct Config {
    rom_path : String,
    #[structopt(name = "wrapping_enabled", help = "Enable sprite wrapping on the borders of the screen (needed by some games, such as BLITZ)", short, long)]
    wrapping_enabled : bool,
    #[structopt(name = "font_path",  help = "Path to the font needed to display information", short, long, default_value = "font.ttf")]
    font_path : String
}

impl Config {
    pub fn rom_path(&self) -> &str {
        &self.rom_path
    }

    pub fn wrapping_enabled(&self) -> bool {
        self.wrapping_enabled
    }

    pub fn font_path(&self) -> &str {
        &self.font_path
    }
}



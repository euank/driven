mod bash;
mod zsh;
use self::bash::Bash;
use self::zsh::Zsh;
use std::io::Write;

pub const SUPPORTED_SHELLS: [&str; 3] = ["zsh", "bash", "fish"];

pub fn from_name(name: &str) -> Option<&dyn Shell> {
    match name {
        "bash" => Some(&Bash),
        "zsh" => Some(&Zsh),
        _ => None,
    }
}

pub trait Shell {
    fn driven_init(&self) -> &'static str;

    fn export_var(&self, cmdfd: &mut dyn Write, name: &str, val: &str) -> Result<(), String>;
}

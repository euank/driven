use super::Shell;

use std::io::Write;

pub struct Zsh;

impl Shell for Zsh {
    fn driven_init(&self) -> &'static str {
        concat!(
            r#"
__driven_add_dir() {
    source <(driven visit --shell zsh "${PWD}")
}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd __driven_add_dir
"#,
        )
    }

    fn export_var(&self, cmdfd: &mut dyn Write, name: &str, val: &str) -> Result<(), String> {
        write!(cmdfd, "export {}={}\n", name, val)
            .map_err(|e| format!("error writing to cmdfd: {}", e))
    }
}

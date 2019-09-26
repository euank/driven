use super::Shell;
use std::io::Write;

pub struct Bash;

impl Shell for Bash {
    fn driven_init(&self) -> &'static str {
        // PROMPT_COMMAND modification inspired by https://github.com/clvv/fasd/blob/90b531a5daaa545c74c7d98974b54cbdb92659fc/fasd#L132-L136
        concat!(
            r#"
__driven_add_dir() {
    # TODO: should driven keep track of this itself in its datadir?
    if [[ "${__DRIVEN_LAST_PWD:-}" != "${PWD}" ]]; then
        source <(driven visit --shell zsh "${PWD}")
    fi
    __DRIVEN_LAST_PWD="${PWD}"
}
"#
        )
    }

    fn export_var(&self, cmdfd: &mut dyn Write, name: &str, val: &str) -> Result<(), String> {
        write!(cmdfd, "export {}={}\n", name, val)
            .map_err(|e| format!("error writing to cmdfd: {}", e))
    }
}

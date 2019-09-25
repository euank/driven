use super::Shell;

pub struct Zsh;

impl Shell for Zsh {
    fn driven_init(&self) -> &'static str {
        concat!(
            r#"
__driven_add_dir() {
    driven visit "${PWD}"
}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd __driven_add_dir
"#,
        )
    }
}

use super::Shell;

pub struct Bash;

impl Shell for Bash {
    fn driven_init(&self) -> &'static str {
        // PROMPT_COMMAND modification inspired by https://github.com/clvv/fasd/blob/90b531a5daaa545c74c7d98974b54cbdb92659fc/fasd#L132-L136
        concat!(
            r#"
__driven_add_dir() {
    # TODO: should driven keep track of this itself in its datadir?
    if [[ "${__DRIVEN_LAST_PWD:-}" != "${PWD}" ]]; then
        driven visit "${PWD}"
    fi
    __DRIVEN_LAST_PWD="${PWD}"
}
"#
        )
    }
}

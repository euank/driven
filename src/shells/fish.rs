use super::Shell;

pub struct Fish;

impl Shell for Fish {
    fn driven_init(&self) -> &'static str {
        concat!(
            r#"
function __driven_preexec --on-variable PWD
    status --is-command-substitution; and return
    driven visit (pwd)
end
"#
        )
    }
}

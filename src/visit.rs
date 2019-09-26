use driven_parser;
use log::debug;
use std::collections::HashMap;
use std::fs;
use std::io::{Write, Read};
use std::path::Path;

use crate::shells::Shell;

#[derive(PartialEq, Debug)]
pub struct VisitResult {
    pub vars: HashMap<String, String>,
    // TODO: shell files to process, etc etc
}

pub fn visit(shell: &dyn Shell, cmdfd: &mut dyn Write, dir: &str) -> Result<(), String> {
    let top_drivenfile = Path::new(dir).join(".driven");
    if !top_drivenfile.exists() {
        debug!("no driven file");
        // TODO: recurse up
        return Ok(());
    }
    let mut f = fs::File::open(top_drivenfile).map_err(|e| format!("cannot open file: {}", e))?;
    let mut data = String::new();
    f.read_to_string(&mut data)
        .map_err(|e| format!("error reading file: {}", e))?;

    let mut exports = HashMap::new();
    let drivenfile = driven_parser::drivenfile(&data)?;
    for var in drivenfile.variables {
        if var.internal {
            continue;
        }
        exports.insert(var.name.resolve(), var.value.resolve());
    }

    for (name, val) in exports {
        shell.export_var(cmdfd, &name, &val)?;
    }
    Ok(())
}

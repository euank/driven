use driven_parser;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path};

#[derive(PartialEq, Debug)]
pub struct VisitResult {
    pub vars: HashMap<String, String>,
    // TODO: shell files to process, etc etc
}

pub fn visit(dir: &str) -> Result<VisitResult, String> {
    let top_drivenfile = Path::new(dir).join(".driven");
    if !top_drivenfile.exists() {
        return Ok(VisitResult{
            vars: HashMap::new(),
        })
    }
    let mut f = fs::File::open(top_drivenfile).map_err(|e| format!("cannot open file: {}", e))?;
    let mut data = String::new();
    f.read_to_string(&mut data).map_err(|e| format!("error reading file: {}", e))?;

    let mut res = HashMap::new();
    let drivenfile = driven_parser::drivenfile(&data)?;
    for var in drivenfile.variables {
        if var.internal {
            continue
        }
        res.insert(var.name.resolve(), var.value.resolve());
    }

    Ok(VisitResult{
        vars: res,
    })
}

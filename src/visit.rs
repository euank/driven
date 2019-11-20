use driven_parser;
use driven_parser::{StringRef, DrivenFile};
use log::debug;
use std::collections::{HashMap, BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path};

use crate::shells::Shell;

struct VisitConfig {
    topdir: Option<String>,
    topdir_metadata: Option<FileModifiedCheck>,
}

enum FileModifiedCheck {
    MtimeAndSize((u64, u64)),
    MD5Sum(String),
}

impl VisitConfig {
    fn from_env() -> Self {
        // DRIVEN_TOPDIR just contains the path of the topdir, e.g.
        // "__DRIVEN_TOPDIR=/home/user/foo" if the top driven file loaded was
        // "/home/user/foo/.driven"
        let topdir = match std::env::var("__DRIVEN_TOPDIR") {
            Ok(s) => Some(s),
            Err(std::env::VarError::NotPresent) => None,
            Err(e) => panic!("{}", e),
        };
        // DRIVEN_MODCHECK contains metadata to allow checking if a driven file was modified,
        // either in the form of the mtime or md5sum of the topdir.
        // It's a string of valid json.
        let topdir_metadata = match std::env::var("__DRIVEN_MODCHECK") {
            Ok(s) => {
                if s == "" {
                    None
                // TODO: parse json, don't use a dumb handrolled format
                } else if s.starts_with("1") {
                    let parts: Vec<_> = s[1..].split(" ").collect();
                    if parts.len() != 2 {
                        panic!("malformed MTIME check in DRIVEN_MODCHECK");
                    }
                    Some(FileModifiedCheck::MtimeAndSize((parts[0].parse().unwrap(), parts[1].parse().unwrap())))
                } else if s.starts_with("2") {
                    Some(FileModifiedCheck::MD5Sum(s[1..].to_string()))
                } else {
                    // could happen on downgrade, so maybe we shouldn't panic
                    panic!("unrecognized MODCHECK format prefix: {}", s);
                }
            },
            Err(std::env::VarError::NotPresent) => None,
            Err(e) => panic!("{}", e),
        };

        VisitConfig{
            topdir: topdir,
            topdir_metadata: topdir_metadata,
        }
    }
}


pub fn visit(shell: &dyn Shell, cmdfd: &mut dyn Write, dir: &str) -> Result<(), String> {
    let res = visit_helper(dir)?;

    let mut exports = BTreeMap::new();
    for var in res.vars {
        exports.insert(var.0, var.1);
    }

    for (name, val) in exports {
        shell.export_var(cmdfd, &name, &val)?;
    }
    Ok(())
}

// VisitResult contains the result of attempting to resolve all driven files from visiting a given
// directory.
#[derive(PartialEq, Debug)]
struct VisitResult {
    internal_vars: BTreeMap<String, String>,
    vars: BTreeMap<String, String>,
}

impl VisitResult {
    fn add_file(&mut self, file: &DrivenFile) -> Result<(), String> {
        let mut unresolved_vars = file.variables.clone();
        while unresolved_vars.len() > 0 {
            let mut missing_vars: Vec<String> = Vec::new();
            let mut resolved_indices = Vec::new();
            for i in 0..unresolved_vars.len() {
                let var = &unresolved_vars[i];
                let resolution_target: HashMap<_, _> = self.internal_vars.clone().into_iter().chain(self.vars.clone()).collect();
                match var.resolve(&resolution_target) {
                    Ok((name, val)) => {
                        if var.internal {
                            debug!("resolved internal var {} = {}", name, val);
                            self.internal_vars.insert(name, val);
                        } else {
                            debug!("resolved var {} = {}", name, val);
                            self.vars.insert(name, val);
                        }
                        resolved_indices.push(i);
                    }
                    Err(e) => {
                        missing_vars.push(e.var);
                    }
                }
            }

            if resolved_indices.len() == 0 {
                return Err(format!("could not resolve variables: {}", missing_vars.join(", ")));
            }
            resolved_indices.reverse();
            for i in resolved_indices {
                unresolved_vars.remove(i);
            }
        }
        Ok(())
    }
}

// visit a given directory and return a structured VisitResult
fn visit_helper(dir: &str) -> Result<VisitResult, String> {
    let mut drivenfiles: Vec<(&Path, _)> = Vec::new();

    let mut remaining_paths = Vec::new();
    let canon_path = Path::new(dir).canonicalize().map_err(|e| format!("could not canonicalize path: {}", e))?;
    remaining_paths.push(canon_path.as_path());

    while remaining_paths.len() > 0 {
        let path = remaining_paths.pop().unwrap();
        debug!("[visit] trying path {}", path.to_string_lossy());
        match load_driven_file(path)? {
            None => {
                if let Some(parent) = path.parent() {
                    remaining_paths.push(parent);
                }
            }
            Some(d) => {
                if !d.ignore_parents {
                    if let Some(parent) = path.parent() {
                        remaining_paths.push(parent);
                    }
                } else {
                    debug!("file at path {} requested we ignore parents; not recursing up further", path.to_string_lossy());
                }
                drivenfiles.push((&path, d));
            }
        }
    }

    let mut visit_result = VisitResult{
        internal_vars: BTreeMap::new(),
        vars: BTreeMap::new(),
    };

    // Now resolve driven files, root first, with deduping.
    let mut resolved = BTreeSet::new();
    drivenfiles.reverse();
    for file in drivenfiles {
        if !resolved.insert(file.0) {
            debug!("skipping {}; already visited", file.0.to_string_lossy());
            // only process files once.
            // e.g. if someone has the structure:
            // /.driven (y = 1)
            // /foo/.driven (include file "/.driven")
            // Then don't re-process /.driven after it was included the first time.
            continue
        }
        debug!("visiting {}", file.0.to_string_lossy());

        visit_result.add_file(&file.1)
            .map_err(|e| format!("error parsing drivenfile {}: {}", file.0.to_string_lossy(), e))?;
    }

    Ok(visit_result)
}

fn load_driven_file<'a, 'b>(path: &Path) -> Result<Option<DrivenFile>, String> {
    let drivenfile = path.join(".driven");
    if !drivenfile.exists() {
        debug!("no driven file");
        return Ok(None)
    }
    let mut f = fs::File::open(drivenfile).map_err(|e| format!("cannot open file: {}", e))?;
    let mut data = String::new();
    f.read_to_string(&mut data)
        .map_err(|e| format!("error reading file: {}", e))?;

    let file = driven_parser::drivenfile(&data)?;
    Ok(Some(file))
}

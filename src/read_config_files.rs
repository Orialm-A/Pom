use serde::Deserialize;
use std::fs;
use crate::errors::{PomErrorCode, PomResult};
use std::io::ErrorKind;

#[derive(Debug, Deserialize)]
pub struct GenerationLayoutEntry {
    pub path: String,
    pub defgroup: Option<String>,
    pub brief: Option<String>,
    #[serde(default)]
    pub contains_modules: bool,
    pub module_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlOutput { dir: Vec<GenerationLayoutEntry> }

pub fn read_generation_layout() -> PomResult<Vec<GenerationLayoutEntry>> {
    let dir_tree_sources: Vec<&str> = vec![
        // Later: add higher priority config files here
        "assets/default_generation_layout.toml", // Lowest priority
    ];

    let mut dir_tree: Option<TomlOutput> = None;

    for dir_tree_source in dir_tree_sources {
        match resolve_dir_tree(dir_tree_source) {
            Ok(extracted_dir_tree) => {
                dir_tree = Some(extracted_dir_tree);
                println!("Start project generation using {}...", dir_tree_source);
                break; // stop at first valid source
            }
            Err((PomErrorCode::GenerationLayoutFileCandidateNotFound, _)) => {
                continue; // try lower priority source
            }
            Err(e) => return Err(e), // Propagate to caller without unpacking
        }
    }

    match dir_tree {
        None => Err((
            PomErrorCode::GenerationLayoutFileCouldNotFindAny,
            None,
        )),
        Some(extracted_dir_tree) => Ok(extracted_dir_tree.dir),
    }
}


fn resolve_dir_tree(dir_tree_path_str: &str) -> PomResult<TomlOutput> {

    let dir_tree_str = match fs::read_to_string(dir_tree_path_str) {
        Ok(extracted_string) => extracted_string,
        Err(src) => {
            return match src.kind() {
                ErrorKind::NotFound => Err((PomErrorCode::GenerationLayoutFileCandidateNotFound, None)),
                _ => Err((
                    PomErrorCode::GenerationLayoutFileCantRead,
                    Some(src.to_string())  // `src` type is `Error` which implement `Display`
                )),
            };
        }
    };

    let dir_tree: TomlOutput = match toml::from_str(&dir_tree_str){
        Ok(extracted_dir_tree) => { extracted_dir_tree },
        Err(src) => {
            return Err((
                PomErrorCode::GenerationLayoutFileCantParse,
                Some(src.to_string())  // `src` type is `Error` which implement `Display`
            ));
        },
    };

    Ok(dir_tree)
}

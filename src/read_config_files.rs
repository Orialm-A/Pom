use serde::Deserialize;
use std::fs;
use crate::errors::{PomErrorCode, PomResult};
use std::io::ErrorKind;

#[derive(Debug, Deserialize)]
pub struct DirSpec {
    pub path: String,
    defgroup: Option<String>,
    brief: Option<String>,
    #[serde(default)]
    contains_modules: bool,
    module_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DirTree { dir: Vec<DirSpec> }

pub fn get_dir_tree() -> PomResult<Vec<DirSpec>> {
    let dir_tree_sources: Vec<&str> = vec![
        // Later: add higher priority config files here
        "assets/default_tree.toml", // Lowest priority
    ];

    let mut dir_tree: Option<DirTree> = None;

    for dir_tree_source in dir_tree_sources {
        match resolve_dir_tree(dir_tree_source) {
            Ok(extracted_dir_tree) => {
                dir_tree = Some(extracted_dir_tree);
                println!("Use {}", dir_tree_source);
                break; // stop at first valid source
            }
            Err((PomErrorCode::ConfigFileDirTreePathNotFound, _)) => {
                continue; // try lower priority source
            }
            Err((error_code, src)) => {
                error_code.handler(src.as_deref());
            }
        }
    }

    match dir_tree {
        None => Err((
            PomErrorCode::ConfigFileNoDirTreeSourcesFound,
            None,
        )),
        Some(extracted_dir_tree) => Ok(extracted_dir_tree.dir),
    }
}


fn resolve_dir_tree(dir_tree_path_str: &str) -> PomResult<DirTree> {

    let dir_tree_str = match fs::read_to_string(dir_tree_path_str) {
        Ok(extracted_string) => extracted_string,
        Err(src) => {
            return match src.kind() {
                ErrorKind::NotFound => Err((PomErrorCode::ConfigFileDirTreePathNotFound, None)),
                _ => Err((
                    PomErrorCode::ConfigFileCantReadDirTree,
                    Some(src.to_string())  // `src` type is `Error` which implement `Display`
                )),
            };
        }
    };

    let dir_tree: DirTree = match toml::from_str(&dir_tree_str){
        Ok(extracted_dir_tree) => { extracted_dir_tree },
        Err(src) => {
            return Err((
                PomErrorCode::ConfigFileCantParseDirTree,
                Some(src.to_string())  // `src` type is `Error` which implement `Display`
            ));
        },
    };

    Ok(dir_tree)
}

use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
struct DirSpec {
    path: String,
    defgroup: Option<String>,
    brief: Option<String>,
    #[serde(default)]
    contains_modules: bool,
    module_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DirTree { dir: Vec<DirSpec> }

pub fn get_dir_tree() {
    // Public API to try default then user-defined tree
    resolve_dir_tree("assets/default_tree.toml");
}

fn resolve_dir_tree(dir_tree_path_str: &str) {
    let dir_tree_str = fs::read_to_string(dir_tree_path_str).expect("failed to read dir tree config");
    let dir_tree: DirTree = toml::from_str(&dir_tree_str).expect("failed to parse dir tree toml");
    println!("{:#?}", dir_tree);
}

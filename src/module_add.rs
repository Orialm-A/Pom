//! "module add" module
//!
//! This Rust module is used to add a C module to a pom project

use crate::errors::{PomResult};
use crate::cli::resolution::{resolve_project_root, resolve_new_module_name};
use crate::project_toml::{resolve_project_toml};


pub fn module_add(
    module_name: Option<String>,
    layer: Option<String>,
    brief: Option<String>,
    details: Option<String>,
    dry_run: bool,
) -> PomResult<()> {

    // Check project
    let project_root = resolve_project_root(None)?;
    let project_toml = resolve_project_toml(&project_root)?;
    println!("{:#?}", project_toml);

    // Resolve user parameters
    let (_, normalized_module_name, upper_normalized_name) = resolve_new_module_name(module_name)?;
    println!("{:#?} / {:#?}", normalized_module_name, upper_normalized_name);


    // Action

    Ok(())
}

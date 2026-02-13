//! "module add" module
//!
//! This Rust module is used to add a C module to a pom project

use crate::errors::{PomResult};
use crate::cli::resolution::{resolve_project_root, resolve_new_module_name, resolve_module_brief, resolve_module_details, resolve_module_level};
use crate::project_toml::{resolve_project_toml};


pub fn module_add(
    module_name: Option<String>,
    level: Option<String>,
    brief: Option<String>,
    details: Option<String>,
    dry_run: bool,
) -> PomResult<()> {

    // Check project
    let project_root = resolve_project_root(None)?;
    let project_toml = resolve_project_toml(&project_root)?;
    println!("{:#?}", project_toml);

    // Resolve user parameters
    let (module_path, module_prefix) = resolve_module_level(level, &project_toml.levels)?;
    println!("{:#?}", module_path);

    let (normalized_module_name, upper_normalized_name) = resolve_new_module_name(module_name, &module_prefix)?;
    println!("{:#?} / {:#?}", normalized_module_name, upper_normalized_name);

    let brief = resolve_module_brief(brief)?;
    let details = resolve_module_details(details)?;

    println!("@brief {}\r\n{}", brief, details);


    // Action

    Ok(())
}

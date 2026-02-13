mod cli;
mod pom_api;
mod project_create;
mod prompt;
mod errors;
mod project_layout;
mod filesystem;
mod template_rendering;
mod module_add;

use clap::Parser;
use crate::cli::parameters::TopLevelCommands::*;
use crate::cli::parameters::ProjectCommands as Proj;
use crate::cli::parameters::ModuleCommands as Mod;

// API
use crate::project_create::project_create;
use crate::module_add::module_add;
use crate::pom_api as api;



fn main() {
    let command_line = cli::parameters::Cli::parse().command;

    let result = match command_line {
        Project { project_command } => match project_command {
            Proj::Create {
                project_name,
                path,
                target,
                dry_run
            } => project_create(project_name, path, target, dry_run),

            Proj::Rename { new_project_name } => api::project_rename_function(new_project_name),
            Proj::Config => api::project_config_function(),
        },
        Module { module_command } => match module_command {
            Mod::Add {
                module_name,
                layer,
                brief,
                details,
                dry_run,
            } => module_add(module_name, layer, brief, details, dry_run),
            cli::parameters::ModuleCommands::Rename { old_module_name, new_module_name } => api::module_rename_function(old_module_name, new_module_name),
            Mod::Remove { module_name } => api::module_remove_function(module_name),
        },
        Clean => api::clean_function(),
        Build => api::build_function(),
        Rebuild => api::rebuild_function(),
        Flash => api::flash_function(),
        Monitor => api::monitor_function(),
        cli::parameters::TopLevelCommands::Config => api::config_function(),
    };

    match result {
        Ok(()) => {},
        Err((error_code, src)) => {error_code.handler(src.as_deref()); }
    }
}

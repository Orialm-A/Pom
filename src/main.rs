mod cli;
mod pom_api;
mod project_create;
mod prompt;
mod errors;
mod read_config_files;
mod filesystem;

use clap::Parser;
use crate::cli::TopLevelCommands::*;
use crate::cli::ProjectCommands as Proj;
use crate::cli::ModuleCommands as Mod;
use crate::pom_api as api;


fn main() {
    let command_line = cli::Cli::parse().command;

    match command_line {
        Project { project_command } => match project_command {
            Proj::Create {
                project_name,
                path,
                target,
                dry_run
            } => api::project_create_function(project_name, path, target, dry_run),
            Proj::Rename { new_project_name } => api::project_rename_function(new_project_name),
            Proj::Config => api::project_config_function(),
        },
        Module { module_command } => match module_command {
            Mod::Add { module_name } => api::module_add_function(module_name),
            cli::ModuleCommands::Rename { old_module_name, new_module_name } => api::module_rename_function(old_module_name, new_module_name),
            Mod::Remove { module_name } => api::module_remove_function(module_name),
        },
        Clean => api::clean_function(),
        Build => api::build_function(),
        Rebuild => api::rebuild_function(),
        Flash => api::flash_function(),
        Monitor => api::monitor_function(),
        cli::TopLevelCommands::Config => api::config_function(),
    }
}

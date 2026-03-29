mod cli;
mod errors;
mod filesystem;
mod module_add;
mod module_rename;
mod pom_api;
mod project_create;
mod project_layout;
mod project_toml;
mod prompt;
mod template_rendering;

use crate::cli::parameters::ModuleCommands as Mod;
use crate::cli::parameters::ProjectCommands as Proj;
use crate::cli::parameters::TopLevelCommands::*;
use clap::Parser;

// API
use crate::module_add::module_add;
use crate::module_rename::module_rename;
use crate::pom_api as api;
use crate::project_create::project_create;

fn main() {
    let command_line = cli::parameters::Cli::parse().command;

    let result = match command_line {
        Project { project_command } => match project_command {
            Proj::Create {
                project_name,
                path,
                target,
                dry_run,
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
                no_prefix,
                dry_run,
            } => module_add(module_name, layer, brief, details, no_prefix, dry_run),

            Mod::Rename {
                old_module_name,
                new_module_name,
                skip_confirmation,
            } => module_rename(old_module_name, new_module_name, skip_confirmation),

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
        Ok(()) => {}
        Err((error_code, src)) => {
            error_code.handler(src.as_deref());
        }
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "pom")]
pub struct Cli {
    #[command(subcommand)]
    pub command: TopLevelCommands,
}

// Top-level commands
#[derive(Subcommand, Debug)]
pub enum TopLevelCommands {
    /// Manage the project
    Project {
        #[command(subcommand)]
        project_command: ProjectCommands,
    },
    /// Manage modules
    Module {
        #[command(subcommand)]
        module_command: ModuleCommands,
    },
    /// Clean build output
    Clean,
    /// Build the project
    Build,
    /// Clean then build the project
    Rebuild,
    /// Flash the target if a binary is available
    Flash,
    /// Open the serial monitor
    Monitor,
    /// Open the cofiguration menu for pom
    Config,
}

#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// Create a new project (name will be prompted of omitted)
    Create { project_name: Option<String> },
    /// Rename the current project. Fail if current directory is not a pom project
    Rename { new_project_name: Option<String> },
    /// Open the configuration menu for the project
    Config,
}

#[derive(Subcommand, Debug)]
pub enum ModuleCommands {
    /// Add a new module (name will be prompted of omitted)
    Add { module_name: Option<String> },
    /// Rename a module (old and new name will be prompted of omitted)
    Rename {
        old_module_name: Option<String>,
        new_module_name: Option<String>
    },
    /// Remove a module (name will be prompted of omitted)
    Remove { module_name: Option<String> },
}

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Create a new project (name and options will be prompted of omitted)
    Create {
        /// Project name
        #[arg(value_name = "NAME")]
        project_name: Option<String>,
        /// Path where the project will be created
        #[arg(short, long, value_name = "PATH")]
        path: Option<PathBuf>,
        /// Specify the project target
        #[arg(short, long, value_name = "TARGET")]
        target: Option<String>,
        /// Print what would be done without creating / modifying files
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Rename the current project. Fail if current directory is not a pom project
    Rename { new_project_name: Option<String> },
    /// Open the configuration menu for the project
    Config,
}

#[derive(Subcommand, Debug)]
pub enum ModuleCommands {
    /// Add a new module (name will be prompted of omitted)
    Add {
        /// Module name
        module_name: Option<String>,
        /// Layer where to add the project
        #[arg(short, long, value_name = "LAYER")]
        layer: Option<String>,
        /// Module documentation `@brief`
        #[arg(short, long, value_name = "BRIEF")]
        brief: Option<String>,
        /// Module documentation `@details`
        #[arg(short, long, value_name = "DETAILS")]
        details: Option<String>,
        /// Create module files without the layer prefix
        #[arg(long)]
        no_prefix: bool,
        /// Print what would be done without creating / modifying files
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    /// Rename a module (old and new name will be prompted of omitted)
    Rename {
        /// Current name of the module to rename
        #[arg(value_name = "OLD_NAME")]
        old_module_name: Option<String>,
        /// New name for the module
        #[arg(value_name = "NEW_NAME")]
        new_module_name: Option<String>,
        /// process with renaming without confirmation prompt
        #[arg(short = 'y', long = "yes")]
        skip_confirmation: bool,
    },
    /// Remove a module (name will be prompted of omitted)
    Remove { module_name: Option<String> },
}

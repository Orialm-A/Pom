use std::env;
use std::path::PathBuf;
use crate::errors::PomErrorCode;
use crate::prompt::{prompt_if_missing_string, slugify_snake};

pub fn project_create(
    // The project name passed in the CLI
    project_name_parameter: Option<String>,
    // The path where to create the project passed in the CLI
    project_path_parameter: Option<PathBuf>,
    // Print what would be done without creating / modifying files
    dry_run: bool
) {
    let project_root = match get_project_root(project_path_parameter) {
        Ok(extracted_project_root) => { extracted_project_root },
        Err(error_code) => { error_code.handler(); }
    };

    match validate_project_root(&project_root) {
        Ok(()) => {},
        Err(error_code) => { error_code.handler(); }
    };

    let (project_name, project_name_normalized) = get_project_name(project_name_parameter);

    let project_dir = project_root.join(&project_name_normalized);

    println!(
        "Creating project '{}' at `{}`...",
        project_name,
        project_dir.display()
    );

    if !dry_run {
        println!("(not implemented) Would create project files");
    }
}


fn get_project_root(project_path_parameter: Option<PathBuf>) -> Result<PathBuf, PomErrorCode> {
    // Path in environment variable is tested first to return early (dev highest priority)
    match env::var("POM_DEV_TEST_PROJECT") {
        Ok(project_root) => return Ok(PathBuf::from(project_root)),
        Err(env::VarError::NotPresent) => {},
        Err(env::VarError::NotUnicode(_)) => {
            return Err(PomErrorCode::ProjectPathDebugNotUnicode);
        },
    };

    // Parameter is tested before local directory to return early ensure user input priority
    match project_path_parameter {
        Some(project_root) => return Ok(project_root),
        None => {},
    };

    // Current directory fallback
    match env::current_dir() {
        Ok(project_root) => Ok(project_root),
        Err(_) => Err(PomErrorCode::ProjectPathCurrentDirFailed),
        //TODO: Find a way to escalate the precisions like `_` here. It would explain why it failed
    }
}


fn validate_project_root(project_root: &PathBuf) -> Result<(), PomErrorCode> {
    let stringified_project_root = project_root.as_os_str().to_string_lossy();

    if stringified_project_root.trim().is_empty() {
        return Err(PomErrorCode::ProjectPathEmpty);
    }

    let marker = project_root.join("pom_source_safeguard.txt");
    if marker.is_file() {
        return Err(PomErrorCode::ProjectPathInPomSource);
    }

    Ok(())
}


fn get_project_name(project_name_parameter: Option<String>) -> (String, String) {
    // `project_name` is to be used in documents read by humans, like README.md
    let project_name = prompt_if_missing_string(project_name_parameter, "Project name");

    // `project_name_normalized` is to be used in paths
    let project_name_normalized = slugify_snake(&project_name);

    (project_name, project_name_normalized)
}

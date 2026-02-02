use std::env;
use std::path::PathBuf;
use std::process;
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
        Err(error_description) => {
            eprintln!("{error_description}");
            process::exit(1);
        }
    };

    match validate_project_root(&project_root) {
        Ok(()) => {},
        Err(error_description) => {
            eprintln!("{error_description}");
            process::exit(1);
        }
    };

    let (project_name, project_name_normalized) = get_project_name(project_name_parameter);

    let project_dir = project_root.join(&project_name_normalized);

    println!(
        "Creating project '{}' at `{}`...",
        project_name,
        project_dir.display()
    );

    if dry_run == false {
        println!("File creating not implemented yet");
    }
}


fn get_project_root(project_path_parameter: Option<PathBuf>) -> Result<PathBuf, String> {
    // Path in environment variable is tested first to return early (dev highest priority)
    match env::var("POM_DEV_TEST_PROJECT") {
        Ok(project_root) => return Ok(PathBuf::from(project_root)),
        Err(env::VarError::NotPresent) => {},
        Err(env::VarError::NotUnicode(_)) => {
            return Err("POM_DEV_TEST_PROJECT contains invalid Unicode.".into());
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
        Err(e) => Err(format!("Cannot get current directory: {e}")),
    }
}


fn validate_project_root(project_root: &PathBuf) -> Result<(), String> {
    let stringified_project_root = project_root.as_os_str().to_string_lossy();

    if stringified_project_root.trim().is_empty() {
        return Err(format!("`{}` is empty or whitespace", project_root.display()));
    }

    let marker = project_root.join("pom_source_safeguard.txt");
    if marker.is_file() {
        return Err(format!("Refusing to use `{}` as project root cause it looks like Pom's source directory. Did you export `POM_DEV_TEST_PROJECT` correctly?", project_root.display()));
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

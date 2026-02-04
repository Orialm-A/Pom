use std::env;
use std::path::{PathBuf, Path};
use crate::errors::{PomErrorCode, PomResult};
use crate::prompt::{prompt_if_missing_string, slugify_snake};
use std::fs;

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
        Err((error_code, src)) => { error_code.handler(src.as_deref()); }
    };

    match validate_project_root(&project_root) {
        Ok(()) => {},
        Err((error_code, src)) => { error_code.handler(src.as_deref()); }
    };

    let (project_name, project_name_normalized) = get_project_name(project_name_parameter);

    let project_dir = project_root.join(&project_name_normalized);

    println!(
        "Creating project '{}' at `{}`...",
        project_name,
        project_dir.display()
    );

    if !dry_run {
        match create_root_dir(&project_dir) {
            Ok(()) => {},
            Err((error_code, src)) => {error_code.handler(src.as_deref()); }
        }
    }
}


fn get_project_root(project_path_parameter: Option<PathBuf>) -> PomResult<PathBuf> {
    // Path in environment variable is tested first to return early (dev highest priority)
    match env::var("POM_DEV_TEST_PROJECT") {
        Ok(project_root) => return Ok(PathBuf::from(project_root)),
        Err(env::VarError::NotPresent) => {},
        Err(env::VarError::NotUnicode(src)) => {
            let details = format!("{:?}", src);  // `OsString`. Doesn't implement `Display`
            return Err((
                PomErrorCode::ProjectPathDebugNotUnicode,
                Some(details),
            ));
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
        Err(src) => {
            Err((PomErrorCode::ProjectPathCurrentDirFailed, Some(src.to_string())))
        },
    }
}


fn validate_project_root(project_root: &Path) -> PomResult<()> {
    let stringified_project_root = project_root.as_os_str().to_string_lossy();

    if stringified_project_root.trim().is_empty() {
        return Err((PomErrorCode::ProjectPathEmpty, None));
    }

    let marker = project_root.join("pom_source_safeguard.txt");
    if marker.is_file() {
        return Err((PomErrorCode::ProjectPathInPomSource, None));
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

fn create_root_dir(project_dir: &Path) -> PomResult<()> {  // `PathBuf` owns memory, `Path` is a borrowed view

    if project_dir.exists() {
        let show_path = format!("Check `{}`.", project_dir.display().to_string());
        if project_dir.is_dir() {
            match project_dir.read_dir() {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        return Err((PomErrorCode::ProjectPathNotEmpty, Some(show_path)));
                    }
                }
                Err(src) => {
                    return Err((
                        PomErrorCode::ProjectPathFailedToReadDir,
                        Some(src.to_string()),
                    ));
                }
            }
        } else {
            // Exists but not a directory
            return Err((PomErrorCode::ProjectPathExistsAndNotDir, Some(show_path)));
        }
    }

    match fs::create_dir_all(project_dir) {
        Ok(()) => Ok(()),
        Err(src) => {
            let details = format!("{:?}", src);
            Err((PomErrorCode::ProjectPathFailedToCreateRoot, Some(details)))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::path::Path;

    // Helper to extract just the error code (keeps asserts clean)
    fn code_of<T>(r: PomResult<T>) -> PomErrorCode {
        match r {
            Ok(_) => panic!("expected Err(..), got Ok(..)"),
            Err((code, _details)) => code,
        }
    }

    #[test]
    fn creates_dir_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("new_project");

        assert!(!target.exists());
        create_root_dir(target.as_path()).unwrap();
        assert!(target.is_dir());
    }

    #[test]
    fn succeeds_if_dir_exists_and_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("empty_dir");

        fs::create_dir_all(&target).unwrap();
        assert!(target.is_dir());

        create_root_dir(target.as_path()).unwrap();
        assert!(target.is_dir());
    }

    #[test]
    fn fails_if_dir_exists_and_not_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("non_empty_dir");

        fs::create_dir_all(&target).unwrap();
        File::create(target.join("something.txt")).unwrap();

        let err_code = code_of(create_root_dir(target.as_path()));
        assert_eq!(err_code, PomErrorCode::ProjectPathNotEmpty);
    }

    #[test]
    fn fails_if_path_exists_and_is_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("not_a_dir");

        File::create(&target).unwrap();
        assert!(target.is_file());

        let err_code = code_of(create_root_dir(target.as_path()));
        assert_eq!(err_code, PomErrorCode::ProjectPathExistsAndNotDir);
    }
}

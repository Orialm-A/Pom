use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::browsing::{EntryKind, validate_dir_entry};
use crate::filesystem::module_search::{ModuleFiles, search_module};
use crate::format_text::{get_const_case, get_slug};
use crate::project_layout::{ModuleLevelSpec, ModuleLevelsMap};
use crate::prompt::{confirm, input_text, select};

use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resolve a project name from CLI input or user prompt.
///
/// If `project_name_parameter` is `None`, the user is prompted to enter a project name.
///
/// # Arguments
/// - `project_name_parameter` - Optional project name provided via CLI
///
/// # Returns
/// A tuple containing:
/// - the project name as entered by the user
/// - the slugified project name
///
/// # Errors
/// - Propagates resolution-related errors encountered during execution
// TESTING: Thin wrapper; not unit-tested directly.
pub fn resolve_project_name(project_name_parameter: Option<String>) -> PomResult<(String, String)> {
    let (project_name_normal, project_name_slug, _) =
        resolve_name(project_name_parameter, "Project name")?;
    Ok((project_name_normal, project_name_slug))
}

/// Resolve a new module name from CLI input or user prompt.
///
/// If `module_name_parameter` is `None`, the user is prompted to enter a module name.
/// If `module_prefix` is `Some(X)`,the slugified module name will take 'x_' at the beginning, the `CONST_CASE` name will take `X_` at the beginning.
/// If the new name is already used by another module in the project, the user is prompted for confirmation.
///
/// # Arguments
/// - `module_name_parameter` - Optional module name provided via CLI
/// - `module_prefix` - Optional layer-related prefix for files name
/// - `project_root` - Path to the project root
/// - `search_scope` - Directories to explore recursively
///
/// # Returns
/// A tuple containing:
/// - the slugified module name
/// - the module name in `CONSTANT_CASE`
///
/// # Errors
/// - Propagates resolution-related errors encountered during execution
pub fn resolve_new_module_name(
    module_name_parameter: Option<String>,
    module_prefix: &Option<String>,
    project_root: &Path,
    search_scope: &HashSet<PathBuf>,
) -> PomResult<(String, String)> {
    let (_, mut normalized_module_name, mut header_guard) =
        resolve_name(module_name_parameter, "Module name")?;

    header_guard = format!("{}_H", header_guard);

    if let Some(module_prefix) = module_prefix {
        normalized_module_name = format!("{}_{}", module_prefix, normalized_module_name);
        header_guard = format!("{}_{}", get_const_case(module_prefix), header_guard);
    }

    let search_result = search_module(&normalized_module_name, project_root, search_scope)?;
    let number_of_results = search_result.len();

    if number_of_results != 0 {
        println!(
            "{} module(s) have been found with the name `{}` in the project:",
            number_of_results, normalized_module_name
        );
        let other_modules_location = search_result.get_all_locations_as_text();
        for module_location in other_modules_location {
            println!(" - {}", module_location);
        }
        let confirm_hint = format!(
            "Do you want to use the name `{}` for this new module?",
            normalized_module_name
        );
        let confirm_name = confirm(&confirm_hint)?;
        if !confirm_name {
            return Err((
                PomErrorCode::ModuleNameAlreadyUsed,
                Some(normalized_module_name),
            ));
        }
    }

    Ok((normalized_module_name, header_guard))
}

/// Resolve an existing module name from CLI input or user prompt.
///
/// If `old_module_name_parameter` is `None`, the user is prompted to enter a module name.
/// The resolved name is then searched in the code base:
/// - if exactly one module is found, it is selected automatically
/// - if multiple modules are found, the user is prompted to select a location
/// - if no module is found, an error is returned
///
/// # Arguments
/// - `old_module_name_parameter` - Optional module name provided via CLI
/// - `project_root` - Path to the project root
/// - `search_scope` - Directories to explore recursively
///
/// # Returns
/// A tuple containing:
/// - the path to the selected module
/// - the slugified module name
/// - the module name in `CONSTANT_CASE`
/// - `*.h` / `*.c` file presence information for the selected module
///
/// # Errors
/// - `PomErrorCode::ModuleRenameNotFound` if no module is found
pub fn resolve_old_module_name(
    old_module_name_parameter: Option<String>,
    project_root: &Path,
    search_scope: &HashSet<PathBuf>,
) -> PomResult<(PathBuf, String, String, ModuleFiles)> {
    let (_, old_name_normalized, old_header_guard) = resolve_name(
        old_module_name_parameter,
        "Module old name (without file extension)",
    )?;

    let search_result = search_module(&old_name_normalized, project_root, search_scope)?;

    let number_of_modules = search_result.len();

    let module_location: PathBuf;

    if number_of_modules == 0 {
        return Err((
            PomErrorCode::ModuleRenameNotFound,
            Some(old_name_normalized),
        ));
    } else if number_of_modules > 1 {
        let available_modules: Vec<String> = search_result.get_all_locations_as_text();
        module_location = select_module(&available_modules)?;
    } else {
        module_location = search_result.get_unique_location()?;
    }
    let module_files = search_result.get_module_files(&module_location)?;
    Ok((
        module_location,
        old_name_normalized,
        old_header_guard,
        module_files,
    ))
}

/// Resolve a name from CLI input or user prompt, and derive normalized variants.
///
/// If `original_name` is `None`, the user is prompted using `prompt_hint`.
/// The resulting name is then transformed into:
/// - a normalized snake_case version (for filesystem paths)
/// - an upper snake_case version (for header guards)
///
/// # Arguments
/// - `original_name` - Optional name provided via CLI
/// - `prompt_hint` - Prompt shown to the user if no name is provided
///
/// # Returns
/// A tuple containing:
/// - the original name (as entered by the user)
/// - the normalized snake_case name
/// - the upper snake_case name
///
/// # Errors
/// - Propagates prompt-related errors encountered during execution
fn resolve_name(
    original_name: Option<String>,
    prompt_hint: &str,
) -> PomResult<(String, String, String)> {
    // `original_name` is the name as typed by user (with whitespaces, emojis...)
    let original_name = input_text(original_name, prompt_hint, false)?;

    // `slug_name` is to be used in paths
    let slug_name = get_slug(&original_name);

    // `const_case_name` is to be used in header guard
    let const_case_name = get_const_case(&slug_name);

    Ok((original_name, slug_name, const_case_name))
}

/// Resolve a Doxygen `@brief` from CLI input or user prompt.
///
/// If `parameter_brief` is `None`, the user is prompted to enter a brief.
/// They may press Enter to leave it empty.
///
/// # Arguments
/// - `parameter_brief` - Optional brief provided via CLI
///
/// # Returns
/// The resolved Doxygen `@brief`, trimmed of leading and trailing whitespace.
///
/// # Errors
/// - Propagates prompt-related errors encountered during execution
// TESTING: Thin wrapper; not unit-tested directly.
pub fn resolve_doxygen_brief(parameter_brief: Option<String>) -> PomResult<String> {
    let brief = input_text(
        parameter_brief,
        "Module brief for Doxygen (Press enter to leave empty)",
        true,
    )?;
    Ok(brief.trim().to_string())
}

/// Resolve Doxygen `@details` from CLI input or user prompt.
///
/// If `parameter_details` is `None`, the user is prompted to enter details.
/// They may press Enter to leave it empty.
///
/// # Arguments
/// - `parameter_details` - Optional details provided via CLI
///
/// # Returns
/// The resolved Doxygen `@details`, trimmed of leading and trailing whitespace.
///
/// # Errors
/// - Propagates prompt-related errors encountered during execution
// TESTING: Thin wrapper; not unit-tested directly.
pub fn resolve_doxygen_details(parameter_details: Option<String>) -> PomResult<String> {
    let details = input_text(
        parameter_details,
        "Module details for Doxygen (Press enter to leave empty)",
        true,
    )?;
    Ok(details.trim().to_string())
}

/// Resolve the project root path from multiple sources.
///
/// Resolution follows this priority:
/// 1. `POM_DEV_TEST_PROJECT` environment variable
/// 2. CLI parameter
/// 3. Current working directory
///
/// Each candidate path is validated before being returned.
///
/// # Arguments
/// - `project_root_parameter` - Optional project root provided via CLI
///
/// # Returns
/// The resolved project root path.
///
/// # Errors
/// - `PomErrorCode::PathToProjectRootEnvVarNotUnicode` if the environment variable is not valid Unicode
/// - `PomErrorCode::PathToProjectRootCantGetCurrentDir` if the current directory cannot be retrieved
pub fn resolve_project_root(project_root_parameter: Option<PathBuf>) -> PomResult<PathBuf> {
    // Path in environment variable is tested first to return early (dev highest priority)
    match env::var("POM_DEV_TEST_PROJECT") {
        Ok(project_root) => {
            let project_root = PathBuf::from(project_root);
            validate_project_root(&project_root)?;
            return Ok(project_root);
        }
        Err(env::VarError::NotPresent) => {}
        Err(env::VarError::NotUnicode(src)) => {
            return Err((
                PomErrorCode::PathToProjectRootEnvVarNotUnicode,
                Some(src.to_string_lossy().to_string()),
            ));
        }
    };

    // Parameter is tested before local directory to return early - ensure user input priority
    if let Some(project_root) = project_root_parameter {
        validate_project_root(&project_root)?;
        return Ok(project_root);
    }

    // Current directory fallback
    match env::current_dir() {
        Ok(project_root) => {
            validate_project_root(&project_root)?;
            Ok(project_root)
        }
        Err(src) => Err((
            PomErrorCode::PathToProjectRootCantGetCurrentDir,
            Some(src.to_string()),
        )),
    }
}

/// Check if a project root is valid.
///
/// # Arguments
/// - `project_root` - Project root Path to validate
///
/// # Returns
/// This function will fire an error on invalid project roots. Else it returns `Ok(())`.
///
/// # Errors
/// - `PomErrorCode::PathToProjectRootEmpty` if the path is empty
/// - `PomErrorCode::PathToProjectRootInPomSource` is within Pom source (dev safety)
fn validate_project_root(project_root: &Path) -> PomResult<()> {
    let stringified_project_root = project_root.as_os_str().to_string_lossy();

    if stringified_project_root.trim().is_empty() {
        return Err((PomErrorCode::PathToProjectRootEmpty, None));
    }

    let marker = project_root.join("pom_source_safeguard.txt");
    if marker.is_file() {
        return Err((PomErrorCode::PathToProjectRootInPomSource, None));
    }

    Ok(())
}

/// Resolve a module level from CLI input or user prompt.
///
/// A module level is one of the project directories where a module can be placed.
/// In the default project layout, these are:
/// - `"app"` (`src/app`)
/// - `"services"`
/// - `"devices"`
/// - `"peripherals"`
/// - `"bsp"`
///
/// If `level_parameter` is `None`, or if it is not found in `project_levels_map`,
/// the user is prompted to select a level.
///
/// # Arguments
/// - `level_parameter` - Optional module level provided via CLI
/// - `project_levels_map` - Mapping of existing module levels, as described in the project's `pom.toml`
///
/// # Returns
/// A tuple containing:
/// - the path to the directory associated with the selected level
/// - the module prefix associated with the selected level
/// - the level name
///
/// # Errors
/// - Propagates prompt-related errors encountered during execution
// TESTING: Thin wrapper; not unit-tested directly.
pub fn resolve_module_level(
    level_parameter: Option<String>,
    project_levels_map: &ModuleLevelsMap,
) -> PomResult<(PathBuf, Option<String>, String)> {
    if let Some(level_parameter) = level_parameter
        && let Some(level_spec) = project_levels_map.get(&level_parameter)
    {
        return Ok((
            level_spec.path.clone(),
            level_spec.prefix.clone(),
            level_parameter,
        ));
    }

    let (selected_level_spec, selected_level_name) = select_module_level(project_levels_map)?;

    Ok((
        selected_level_spec.path.clone(),
        selected_level_spec.prefix.clone(),
        selected_level_name,
    ))
}

/// Resolve a project target from CLI input or available targets on disk.
///
/// Targets are discovered by scanning predefined directories (currently `assets/target`).
/// Each subdirectory represents a selectable target.
///
/// If `project_target_parameter` is provided and matches an available target,
/// it is selected automatically. Otherwise, the user is prompted to choose
/// from the list of discovered targets.
///
/// # Arguments
/// - `project_target_parameter` - Optional target name provided via CLI
///
/// # Returns
/// The path to the selected target directory.
///
/// # Errors
/// - Propagates filesystem- and prompt-related errors encountered during execution
pub fn resolve_project_target(project_target_parameter: Option<String>) -> PomResult<PathBuf> {
    const DEFAULT_SOURCE: &str = "assets/target";
    let target_files_sources: [&str; 1] = [DEFAULT_SOURCE];

    let mut available_targets: HashMap<String, PathBuf> = HashMap::new();

    for source_str in target_files_sources {
        let source_path = Path::new(source_str);

        for entry in WalkDir::new(source_path)
            .max_depth(1)
            .min_depth(1)
            .into_iter()
        {
            // Depth = 0: Source itself
            // Depth = 1: target directories, what must be selected
            // Depth > 1: Target direcotires content

            let validated_entry = validate_dir_entry(entry, source_path)?;

            let entry_full_path = validated_entry.full_path;
            let name = validated_entry.relative_path.to_string_lossy().to_string();

            match validated_entry.kind {
                EntryKind::File => {
                    continue;
                } // Look for a target-specific directory
                EntryKind::Symlink => {
                    println!(
                        "WARNING: `{}` is a symlink. It is not supported by pom current version.",
                        name
                    );
                    continue;
                    // Doesn't justify an error
                }
                EntryKind::Directory => {
                    if let Some(ref wanted) = project_target_parameter {
                        // Borrow the value inside `Some` instead of moving it
                        if &name == wanted {
                            return Ok(entry_full_path);
                        }
                    }

                    let available_target_name = match source_str {
                        DEFAULT_SOURCE => format!("{} (default)", name),
                        _ => format!("{} (user)", name),
                    };
                    available_targets.insert(available_target_name, entry_full_path);
                }
            };
        }
    }

    select_target(&available_targets)
}

/// Prompt the user to select a target from a menu
///
/// # Arguments
/// - `available_targets` - Mapping of target names to paths
///
/// # Returns
/// Path to the selected target.
///
/// # Errors
/// - `PomErrorCode::FileTemplateMissing` if the map is empty
fn select_target(available_targets: &HashMap<String, PathBuf>) -> PomResult<PathBuf> {
    let keys: Vec<String> = available_targets.keys().cloned().collect();

    if keys.is_empty() {
        return Err((
            PomErrorCode::FileTemplateMissing,
            Some("In `assets/target`".to_string()),
        ));
    }

    let selected_key = select(
        &keys,
        "Typed target not found. Select one of the available targets",
    )?;
    Ok(available_targets[&selected_key].clone())
}

/// Prompt the user to select a module level from a menu.
///
/// # Arguments
/// - `available_targets` - Mapping of module level names to their specs
///
/// # Returns
/// Specs of input_textthe selected module level.
///
/// # Errors
/// - Propagates promt-relate errors encountered during execution
fn select_module_level(available_levels: &ModuleLevelsMap) -> PomResult<(ModuleLevelSpec, String)> {
    let keys: Vec<String> = available_levels.keys().cloned().collect();
    let selected_key = select(
        &keys,
        "Typed level not found. Select one from availables in `pom.toml`",
    )?;
    Ok((available_levels[&selected_key].clone(), selected_key))
}

/// Prompt the user to select a module location from a menu.
///
/// # Arguments
/// - `available_modules` - Paths to the available module locations
///
/// # Returns
/// Path to the selected module location.
///
/// # Errors
/// - Propagates promt-relate errors encountered during execution
fn select_module(available_modules: &[String]) -> PomResult<PathBuf> {
    let selected_key = select(
        available_modules,
        "Several modules found. Select the correct location",
    )?;

    Ok(selected_key.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::io_ops::{ExistingFilePolicy, create_directories, write_file};
    use tempfile::tempdir;

    mod new_module_name_resolution_tests {
        use super::*;

        #[test]
        fn module_name_with_prefix() {
            let result =
                resolve_new_module_name(Some("Test Name".to_string()), &Some("a".to_string()));
            assert!(result.is_ok());

            let (module_name, header_guard) = result.unwrap();

            assert_eq!(module_name, "a_test_name".to_string());
            assert_eq!(header_guard, "A_TEST_NAME_H".to_string());
        }

        #[test]
        fn module_name_without_prefix() {
            let result = resolve_new_module_name(Some("Test Name".to_string()), &None);
            assert!(result.is_ok());

            let (module_name, header_guard) = result.unwrap();

            assert_eq!(module_name, "test_name".to_string());
            assert_eq!(header_guard, "TEST_NAME_H".to_string());
        }
    }

    mod old_module_name_resolution_tests {
        use super::*;

        fn create_test_file_tree() -> (tempfile::TempDir, PathBuf, HashSet<PathBuf>) {
            let temp_dir = tempdir().unwrap();
            let project_root = temp_dir.path().join("project_root");

            // let directories_collection: Vec<PathBuf> = Vec::from([
            let directories_collection = [
                project_root.join("src/services"),
                project_root.join("src/peripherals"),
                project_root.join("unit_tests"),
            ];
            create_directories(&directories_collection).unwrap();

            let files_collection = [
                project_root.join("src/services/timer.h"),
                project_root.join("src/services/timer.c"),
                project_root.join("src/peripherals/timer.c"),
                project_root.join("src/peripherals/tim.h"),
                project_root.join("unit_tests/timer.h"),
                project_root.join("unit_tests/tests_timer.c"),
            ];

            for file in files_collection {
                write_file(&file, "", &ExistingFilePolicy::Overwrite).unwrap();
            }

            let mut search_scope = HashSet::new();
            search_scope.insert(PathBuf::from("src"));
            search_scope.insert(PathBuf::from("src"));
            (temp_dir, project_root, search_scope)
        }

        #[test]
        fn error_when_module_not_found() {
            let (_temp_dir, project_root, search_scope) = create_test_file_tree();
            let old_module_name = Some(String::from("unexistent_module"));
            let (error_code, _) =
                resolve_old_module_name(old_module_name, &project_root, &search_scope).unwrap_err();

            assert_eq!(error_code, PomErrorCode::ModuleRenameNotFound);
        }

        #[test]
        fn auto_select_when_module_is_unique() {
            let (_temp_dir, project_root, search_scope) = create_test_file_tree();
            let old_module_name = Some(String::from("tim")); // typing prompt ignored
            let (module_location, _, _, _) =
                resolve_old_module_name(old_module_name, &project_root, &search_scope).unwrap(); // Selection prompt may fire

            assert_eq!(module_location, PathBuf::from("src/peripherals"));
        }
    }

    #[test]
    fn name_resolution_test() {
        let result = resolve_name(Some("Test Name".to_string()), "Test Hint");
        assert!(result.is_ok());

        let (name, name_normalized, name_const_case) = result.unwrap();
        assert_eq!(name, "Test Name".to_string());
        assert_eq!(name_normalized, "test_name".to_string());
        assert_eq!(name_const_case, "TEST_NAME".to_string());
    }

    mod new_project_root_resolution_tests {
        use super::*;

        #[test]
        fn uses_parameter_when_env_not_set() {
            let temp = tempdir().unwrap();
            let project_root = temp.path().to_path_buf();

            let result = resolve_project_root(Some(project_root.clone())).unwrap();

            assert_eq!(result, project_root);
        }

        #[test]
        fn uses_current_dir_when_no_env_or_param() {
            let temp = tempdir().unwrap();
            let expected = temp.path().to_path_buf();

            let old_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(&expected).unwrap();

            let result = resolve_project_root(None).unwrap();

            assert_eq!(result, expected);

            // restore
            std::env::set_current_dir(old_dir).unwrap();
        }

        #[test]
        fn project_root_empty_rejected() {
            let (error_code, _) = validate_project_root(Path::new("")).unwrap_err();
            assert_eq!(error_code, PomErrorCode::PathToProjectRootEmpty);
        }
    }

    mod target_resolution_tests {
        use super::*;

        #[test]
        fn auto_select_target_when_parameter_matches() {
            let temp = tempdir().unwrap();
            let root = temp.path();

            let target_path_relative = PathBuf::from("assets/target/my_target");
            let target_path_absolute = root.join(&target_path_relative);
            create_directories(&target_path_absolute).unwrap();

            // Change current dir so relative path "assets/target" works
            let old_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(root).unwrap();

            let result = resolve_project_target(Some("my_target".to_string())).unwrap();

            assert_eq!(result, target_path_relative);

            std::env::set_current_dir(old_dir).unwrap();
        }

        #[test]
        fn ignore_files_in_target_directory() {
            let temp = tempdir().unwrap();
            let root = temp.path();

            let valid_target_path_relative = PathBuf::from("assets/target/valid_target");
            let valid_target_path_absolute = root.join(&valid_target_path_relative);
            create_directories(&valid_target_path_absolute).unwrap();

            // File should be ignored
            let file = valid_target_path_absolute.join("not_a_target.txt");
            write_file(&file, "", &ExistingFilePolicy::Overwrite).unwrap();

            let old_dir = std::env::current_dir().unwrap();
            std::env::set_current_dir(root).unwrap();

            let result = resolve_project_target(Some("valid_target".to_string())).unwrap();

            assert_eq!(result, valid_target_path_relative);

            std::env::set_current_dir(old_dir).unwrap();
        }
    }
}

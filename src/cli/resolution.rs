use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::{EntryKind, ModuleFiles, search_module, validate_dir_entry};
use crate::project_layout::ModuleLevelsMap;
use crate::prompt::{
    prompt_if_missing_string, select_module, select_module_level, select_target, slugify_snake,
};

use convert_case::{Case, Casing};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resolve project name passed by parameter in the CLI
pub fn resolve_project_name(
    project_name_parameter: Option<String>,
) -> PomResult<(String, String, String)> {
    resolve_new_name(project_name_parameter, "Project name")
}

/// Resolve new module name passed by parameter in the CLI
pub fn resolve_new_module_name(
    module_name_parameter: Option<String>,
    module_prefix: &Option<String>,
) -> PomResult<(String, String)> {
    let (_, mut normalized_module_name, mut header_guard) =
        resolve_new_name(module_name_parameter, "Module name")?;

    header_guard = format!("{}_H", header_guard);

    if let Some(module_prefix) = module_prefix {
        normalized_module_name = format!("{}_{}", module_prefix, normalized_module_name);
        header_guard = format!("{}_{}", module_prefix.to_case(Case::Constant), header_guard);
    }

    Ok((normalized_module_name, header_guard))
}

/// Resolve old module name passed by parameter in the CLI
pub fn resolve_old_module_name(
    old_module_name_parameter: Option<String>,
    project_root: &Path,
    search_scope: &HashSet<PathBuf>,
) -> PomResult<(PathBuf, String, String, ModuleFiles)> {
    let (_, old_name_normalized, old_header_guard) = resolve_new_name(
        old_module_name_parameter,
        "Module old name (without file extension)",
    )?;

    let search_result = search_module(&old_name_normalized, project_root, search_scope)?;

    let number_of_modules = search_result.get_number_of_modules();

    let module_location: PathBuf;

    if number_of_modules == 0 {
        return Err((
            PomErrorCode::ModuleRenameNotFound,
            Some(old_name_normalized),
        ));
    } else if number_of_modules > 1 {
        module_location = select_module(&search_result)?;
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

fn resolve_new_name(
    original_name: Option<String>,
    prompt_hint: &str,
) -> PomResult<(String, String, String)> {
    // `original_name` is the name as typed by user (with whitespaces, emojis...)
    let original_name = prompt_if_missing_string(original_name, prompt_hint, false)?;

    // `normalized_name` is to be used in paths
    let normalized_name = slugify_snake(&original_name);

    // `upper_normalized_name` is to be used for modules header guard
    let upper_normalized_name = normalized_name.to_case(Case::Constant);

    Ok((original_name, normalized_name, upper_normalized_name))
}

/// Resolve project target passed by parameter in the CLI
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

/// Resolve project root passed by parameter in the CLI
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

    // Parameter is tested before local directory to return early ensure user input priority
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

/// Resolve module brief passed by parameter in the CLI for Doxygen header
pub fn resolve_module_brief(parameter_brief: Option<String>) -> PomResult<String> {
    let brief = prompt_if_missing_string(
        parameter_brief,
        "Module brief for Doxygen (Press enter to leave empty)",
        true,
    )?;
    Ok(brief.trim().to_string())
}

/// Resolve module details passed by parameter in the CLI for Doxygen header
pub fn resolve_module_details(parameter_details: Option<String>) -> PomResult<String> {
    let details = prompt_if_missing_string(
        parameter_details,
        "Module details for Doxygen (Press enter to leave empty)",
        true,
    )?;
    Ok(details.trim().to_string())
}

/// Resolve module level passed by parameter in the CLI for Doxygen group and file location
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_names_are_resolved() {
        let result = resolve_new_name(Some("Test Name".to_string()), "Test Hint");
        assert!(result.is_ok());

        let (name, name_normalized, name_const_case) = result.unwrap();
        assert_eq!(name, "Test Name".to_string());
        assert_eq!(name_normalized, "test_name".to_string());
        assert_eq!(name_const_case, "TEST_NAME".to_string());
    }

    #[test]
    fn module_name_with_prefix() {
        let result = resolve_new_module_name(Some("Test Name".to_string()), &Some("a".to_string()));
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

    #[test]
    fn project_root_empty_rejected() {
        let (error_code, _) = validate_project_root(Path::new("")).unwrap_err();
        assert_eq!(error_code, PomErrorCode::PathToProjectRootEmpty);
    }
}

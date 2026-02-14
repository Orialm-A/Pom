use crate::prompt::{prompt_if_missing_string, slugify_snake, select_target, select_module_level};
use crate::errors::{PomResult, PomErrorCode};
use crate::filesystem::{EntryKind, validate_dir_entry};
use crate::project_layout::{ModuleLevelsMap};

use std::env;
use std::path::{PathBuf, Path};
use std::collections::HashMap;
use walkdir::{WalkDir};
use convert_case::{Case, Casing};


pub fn resolve_project_name(project_name_parameter: Option<String>) -> PomResult<(String, String, String)> {
    resolve_new_name(project_name_parameter, "Project name")
}

pub fn resolve_new_module_name(module_name_parameter: Option<String>, module_prefix: &Option<String>) -> PomResult<(String, String)> {
    let (_, mut normalized_module_name, mut header_guard) = resolve_new_name(module_name_parameter, "Module name")?;

    if let Some(module_prefix) = module_prefix {
        normalized_module_name = format!("{}_{}", module_prefix, normalized_module_name);
        header_guard = format!("{}_{}_H", module_prefix.to_case(Case::Constant), header_guard);
    }

    Ok((normalized_module_name, header_guard))
}


fn resolve_new_name(original_name: Option<String>, prompt_hint: &str) -> PomResult<(String, String, String)> {
    // `original_name` is the name as typed by user (with whitespaces, emojis...)
    let original_name = prompt_if_missing_string(original_name, prompt_hint, false)?;

    // `normalized_name` is to be used in paths
    let normalized_name = slugify_snake(&original_name);

    // `upper_normalized_name` is to be used for modules header guard
    let upper_normalized_name = normalized_name.to_case(Case::Constant);

    Ok((original_name, normalized_name, upper_normalized_name))
}


pub fn resolve_project_target(project_target_parameter: Option<String>) -> PomResult<PathBuf> {
    const DEFAULT_SOURCE: &'static str= "assets/target";
    let target_files_sources: [&str; 1] = [
        DEFAULT_SOURCE,
    ];

    let mut available_targets: HashMap<String, PathBuf> = HashMap::new();

    for source_str in target_files_sources {
        let source_path = Path::new(source_str);

        for entry in WalkDir::new(source_path).max_depth(1).min_depth(1).into_iter() {
            // Depth = 0: Source itself
            // Depth = 1: target directories, what must be selected
            // Depth > 1: Target direcotires content

            let validated_entry = validate_dir_entry(entry, source_path)?;

            let entry_full_path = validated_entry.full_path;
            let name = validated_entry.relative_path.to_string_lossy().to_string();

            match validated_entry.kind {
                EntryKind::File => { continue; }, // Look for a target-specific directory
                EntryKind::Symlink => {
                    println!("WARNING: `{}` is a symlink. It is not supported by pom current version.", name);
                    continue;
                    // Doesn't justify an error
                },
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
                    available_targets.insert(
                        available_target_name,
                        entry_full_path,
                    );
                },
            };
        }
    }

    return select_target(&available_targets);
}


pub fn resolve_project_root(project_root_parameter: Option<PathBuf>) -> PomResult<PathBuf> {

    // Path in environment variable is tested first to return early (dev highest priority)
    match env::var("POM_DEV_TEST_PROJECT") {
        Ok(project_root) => {
            let project_root = PathBuf::from(project_root);
            validate_project_root(&project_root)?;
            return Ok(project_root);
        },
        Err(env::VarError::NotPresent) => {},
        Err(env::VarError::NotUnicode(src)) => {
            return Err((
                PomErrorCode::PathToProjectRootEnvVarNotUnicode,
                Some(format!("{}", src.to_string_lossy().to_string())),
            ));
        },
    };

    // Parameter is tested before local directory to return early ensure user input priority
    if let Some(project_root) = project_root_parameter {
        validate_project_root(&project_root)?;
        return Ok(project_root);
    }

    // Current directory fallback
    match env::current_dir() {
        Ok(project_root) => {
            let project_root = PathBuf::from(project_root);
            validate_project_root(&project_root)?;
            return Ok(project_root);
        },
        Err(src) => {
            Err((PomErrorCode::PathToProjectRootCantGetCurrentDir, Some(src.to_string())))
        },
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


pub fn resolve_module_brief(parameter_brief: Option<String>) -> PomResult<String> {
    let brief = prompt_if_missing_string(parameter_brief, "Module brief for Doxygen (Press enter to leave empty)", true)?;
    Ok(brief.trim().to_string())
}


pub fn resolve_module_details(parameter_details: Option<String>) -> PomResult<String> {
    let details = prompt_if_missing_string(parameter_details, "Module details for Doxygen (Press enter to leave empty)", true)?;
    Ok(details.trim().to_string())
}


pub fn resolve_module_level(level_parameter: Option<String>, project_levels_map: &ModuleLevelsMap) -> PomResult<(String, Option<String>, String)> {
    if let Some(level_parameter) = level_parameter {
        if let Some(level_spec) = project_levels_map.get(&level_parameter) {
            return Ok((level_spec.path.clone(), level_spec.prefix.clone(), level_parameter));
        }
    }

    let (selected_level_spec, selected_level_name) = select_module_level(project_levels_map)?;

    Ok((selected_level_spec.path.clone(), selected_level_spec.prefix.clone(), selected_level_name))
}

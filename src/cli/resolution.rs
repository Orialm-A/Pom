use crate::prompt::{prompt_if_missing_string, slugify_snake, select_target};
use crate::errors::{PomResult, PomErrorCode};

use std::path::{PathBuf, Path};
use std::collections::HashMap;
use std::fs;



pub fn resolve_project_name(project_name_parameter: Option<String>) -> (String, String) {
    // `project_name` is to be used in documents read by humans, like README.md
    let project_name = prompt_if_missing_string(project_name_parameter, "Project name");

    // `project_name_normalized` is to be used in paths
    let project_name_normalized = slugify_snake(&project_name);

    (project_name, project_name_normalized)
}


pub fn resolve_project_target(project_target_parameter: Option<String>) -> PomResult<PathBuf> {
    let project_target_parameter = match project_target_parameter {
        Some(project_target_parameter) => project_target_parameter,
        None => "".to_string(),
    };

    let default_source: &'static str= "assets/target";
    let target_files_sources: [&str; 1] = [
        default_source,
    ];

    let mut available_targets: HashMap<String, PathBuf> = HashMap::new();

    for source_str in target_files_sources {
        let source_path = Path::new(source_str);
        let entries =  match fs::read_dir(source_path) {
            Ok(entries) => entries,  // Shadowing
            Err(src) => {
                return Err((
                    PomErrorCode::TargetFilesSourceReadFail,  // HERE - ERROR 71
                    Some(src.to_string()),
                ))
            }
        };

        for entry in entries {
            let entry = match entry {  // Shadowing
                Ok(entry) => entry,  // Shadowing
                Err(src) => {
                    return Err((
                        PomErrorCode::TargetFilesInvalidEntry,
                        Some(src.to_string()),
                    ));
                }
            };

            let entry_path = entry.path();

            if entry_path.is_file() {
                continue;  // Look for a target-specific directory
            }

            let dir_name = match entry_path.file_name() {
                Some(dir_name) => dir_name,
                None => continue,  // Can't happen with default files
            };

            let dir_name = dir_name.to_string_lossy().to_string();

            if dir_name == project_target_parameter {
                return Ok(entry_path);
            } else {
                let entry_name = match source_str {
                    default_source => format!("{} (default)", dir_name),
                    _ => format!("{} (user)", dir_name),
                };

                available_targets.insert(
                    entry_name,
                    entry_path,
                );
            }
        }
    }

    select_target(&available_targets)
}

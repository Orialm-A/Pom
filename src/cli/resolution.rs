use crate::prompt::{prompt_if_missing_string, slugify_snake, select_target};
use crate::errors::{PomResult};
use crate::filesystem::{EntryKind, validate_dir_entry};

use std::path::{PathBuf, Path};
use std::collections::HashMap;
use walkdir::{WalkDir};



pub fn resolve_project_name(project_name_parameter: Option<String>) -> PomResult<(String, String)> {
    // `project_name` is to be used in documents read by humans, like README.md
    let project_name = prompt_if_missing_string(project_name_parameter, "Project name")?;

    // `project_name_normalized` is to be used in paths
    let project_name_normalized = slugify_snake(&project_name);

    Ok((project_name, project_name_normalized))
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

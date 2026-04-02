use std::path::{Path, PathBuf};

use crate::cli::resolution::{resolve_project_name, resolve_project_root, resolve_project_target};
use crate::filesystem::io_ops::{ExistingFilePolicy, copy_files, create_directories, write_file};

use crate::errors::{PomErrorCode, PomResult};
use crate::project_layout::{DoxygenGroup, resolve_project_layout};
use crate::project_toml::create_pom_toml_file;
use crate::template_rendering::{FieldKey, TemplateFields};

/// Main function to create a project
pub fn project_create(
    project_name_parameter: Option<String>,
    project_root_parameter: Option<PathBuf>,
    project_target_parameter: Option<String>,
    dry_run: bool,
) -> PomResult<()> {
    // Resolve user parameters
    let project_root = resolve_project_root(project_root_parameter)?;
    let (project_name, project_name_normalized) = resolve_project_name(project_name_parameter)?;
    let project_root = project_root.join(&project_name_normalized);

    let project_target = resolve_project_target(project_target_parameter)?;

    let mut rendering_fields = TemplateFields::new();
    rendering_fields.insert(FieldKey::ProjectName, project_name);
    rendering_fields.insert(FieldKey::ProjectNameNormalized, project_name_normalized);

    // Resolve project layout
    let resolved_project_layout = resolve_project_layout(&project_root)?;

    // Action
    println!(
        "Create project directory at `{}`...",
        project_root.display()
    );
    if !dry_run {
        create_root_dir(&project_root)?;
    }

    println!("Create subdirectories...");
    if !dry_run {
        create_directories(&resolved_project_layout.dirs)?;
    }

    println!("Create `doc_groups.h`...");
    if !dry_run {
        create_doc_groups_file(&project_root, &resolved_project_layout.doxygen_groups)?;
    }

    println!("Create `pom.toml`...");
    if !dry_run {
        create_pom_toml_file(&project_root, &resolved_project_layout.module_levels)?;
    }

    println!("Create target-free files...");
    if !dry_run {
        copy_target_free_files(&project_root)?;
    }

    println!("Create target-specific files...");
    if !dry_run {
        copy_files(
            &project_target,
            &project_root,
            ExistingFilePolicy::Fail,
            &Some(rendering_fields),
        )?;
    }

    Ok(())
}

fn create_root_dir(project_root: &Path) -> PomResult<()> {
    // `PathBuf` owns memory, `Path` is a borrowed view

    if project_root.exists() {
        let show_path = format!("Check `{}`.", project_root.display());
        if project_root.is_dir() {
            match project_root.read_dir() {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        return Err((PomErrorCode::PathToProjectRootNotEmpty, Some(show_path)));
                    }
                }
                Err(src) => {
                    return Err((
                        PomErrorCode::PathToProjectRootFailedToReadDir,
                        Some(src.to_string()),
                    ));
                }
            }
        } else {
            // Exists but not a directory
            return Err((
                PomErrorCode::PathToProjectRootExistsAndNotDir,
                Some(show_path),
            ));
        }
    }

    match create_directories(project_root) {
        Ok(()) => Ok(()),
        Err((_, src)) => Err((PomErrorCode::PathToProjectRootFailedToCreateRoot, src)),
    }
}

fn create_doc_groups_file(project_root: &Path, groups_list: &[DoxygenGroup]) -> PomResult<()> {
    let doc_groups_file_path = project_root.join("doc_groups.h");

    let mut doc_groups_file_content = String::new();
    for group in groups_list {
        doc_groups_file_content.push_str(&create_group_block(group));
    }

    write_file(
        &doc_groups_file_path,
        &doc_groups_file_content,
        &ExistingFilePolicy::Fail,
    )
}

fn create_group_block(group: &DoxygenGroup) -> String {
    let mut group_block = String::new();
    group_block.push_str("/**\n");
    let id = &group.name;

    let defgroup = match &group.defgroup {
        Some(t) => t.as_str(),
        None => id.as_str(),
    };
    group_block.push_str(&format!(" * @defgroup {} {}\n", id, defgroup));

    if let Some(brief) = &group.brief {
        group_block.push_str(&format!(" * @brief {}\n", brief));
    }

    group_block.push_str(" */\n\n");

    group_block
}

fn copy_target_free_files(project_root: &Path) -> PomResult<()> {
    let default_source: &'static str = "assets/target_free";

    let files_sources: Vec<&str> = vec![
        // Later: add higher priority config files here
        default_source, // Lowest priority
    ];

    for files_source in files_sources {
        let source_path = std::path::Path::new(files_source);

        if !source_path.exists() && files_source == default_source {
            return Err((
                PomErrorCode::FileTemplateMissing,
                Some(format!("In `{}`", files_source)),
            ));
        }

        match copy_files(source_path, project_root, ExistingFilePolicy::Fail, &None) {
            Ok(file_count) => {
                if file_count == 0 {
                    if files_source == default_source {
                        // Should never get here!
                        // If no file has been copied from the default source, it means `assets/target_free` is empty: The repository has an issue, or the build output has an issue.
                        return Err((
                            PomErrorCode::FileTemplateMissing,
                            Some(format!("In `{}`", files_source)),
                        ));
                    } else {
                        continue; // High priority empty, fall back to lower
                    }
                }
                return Ok(());
            }
            Err(e) => return Err(e), // Propagate to caller without unpacking
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Read;

    mod root_creation {
        use super::*;

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
            assert_eq!(err_code, PomErrorCode::PathToProjectRootNotEmpty);
        }

        #[test]
        fn fails_if_path_exists_and_is_file() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("not_a_dir");

            File::create(&target).unwrap();
            assert!(target.is_file());

            let err_code = code_of(create_root_dir(target.as_path()));
            assert_eq!(err_code, PomErrorCode::PathToProjectRootExistsAndNotDir);
        }
    }

    mod doxygen_file_creation {
        use super::*;

        #[test]
        fn group_block_contains_expected_tags() {
            let group = DoxygenGroup {
                name: "app".into(),
                defgroup: Some("Application Layer".into()),
                brief: Some("High-level behavior.".into()),
            };

            let block = create_group_block(&group);

            // Keep assertions loose (format can evolve)
            assert!(block.contains("@defgroup"));
            assert!(block.contains("app"));
            assert!(block.contains("Application Layer"));
            assert!(block.contains("@brief"));
            assert!(block.contains("High-level behavior."));
        }

        #[test]
        fn writes_doc_groups_file() {
            let tmp = tempfile::tempdir().unwrap();
            let project_root = tmp.path();

            let groups = vec![
                DoxygenGroup {
                    name: "app".into(),
                    defgroup: Some("Application Layer".into()),
                    brief: Some("High-level behavior.".into()),
                },
                DoxygenGroup {
                    name: "hld".into(),
                    defgroup: Some("High-Level Drivers".into()),
                    brief: None,
                },
            ];

            create_doc_groups_file(project_root, &groups).unwrap();

            let mut contents = String::new();
            File::open(project_root.join("doc_groups.h"))
                .unwrap()
                .read_to_string(&mut contents)
                .unwrap();

            assert!(contents.contains("@defgroup"));
            assert!(contents.contains("app"));
            assert!(contents.contains("hld"));
        }
    }
}

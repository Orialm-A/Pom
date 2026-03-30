//! Filesystem I/O operations.
//!
//! Provides utilities for reading, writing, copying, and modifying filesystem entries.

use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::browsing::{EntryKind, validate_dir_entry};
use crate::filesystem::path_list;
use crate::template_rendering::{TemplateFields, get_rendered_template_dest, render_template};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use walkdir::WalkDir;

/// Represents the action to take when a file write or copy conflicts with an existing file
#[derive(PartialEq)]
pub enum ExistingFilePolicy {
    /// The existing file will be lost
    Overwrite,
    /// The process will fail and fire an error
    Fail,
}

/// Create the given directories.
///
/// Existing directories are ignored.
/// Parents are created as needed.
/// This function is idempotent.
///
/// # Arguments
/// - `dirs_paths` - Directories to create
///
/// # Errors
/// - `PomErrorCode::FilesystemDirCreationFail` if a directory creation fails due to an OS error
pub fn create_directories(dirs_paths: &(impl path_list::PathList + ?Sized)) -> PomResult<()> {
    // Read parameter type as "A reference to a type implementing `PathList`"
    for dir_path in dirs_paths.iter_paths() {
        match fs::create_dir_all(dir_path) {
            Ok(()) => continue,
            Err(src) => {
                let details = format!("{}: {:?}", dir_path.display(), src);
                return Err((PomErrorCode::FilesystemDirCreationFail, Some(details)));
            }
        };
    }
    Ok(())
}

/// Copy files from source_root to destination_root.
///
/// Creates subdirectories as needed to preserve the source directory structure.
///
/// # Arguments
/// - `source_root` - Path from which the files must be copied
/// - `destination_root` - Path to which the files must be copied
/// - `existing_file_policy` - What to do if a file already exists at the destination
/// - `fields` optional fields content to copy template files with rendering
///
/// # Returns
/// Number of files copied.
///
/// # Errors
/// - `PomErrorCode::FilesystemFileOverwriteForbidden` if a file already exists and `existing_file_policy` is `ExistingFilePolicy::Fail`
/// - `PomErrorCode::FilesystemCopyFail` if a copy fails due to an OS error
pub fn copy_files(
    source_root: &Path,
    destination_root: &Path,
    existing_file_policy: ExistingFilePolicy,
    fields: &Option<TemplateFields>,
) -> PomResult<usize> {
    if !source_root.exists() {
        return Err((
            PomErrorCode::FilesystemCopySourceMissing,
            Some(source_root.display().to_string()),
        ));
    }
    if !source_root.is_dir() {
        return Err((
            PomErrorCode::FilesystemCopySourceNotDir,
            Some(source_root.display().to_string()),
        ));
    }

    let mut copy_count: usize = 0;

    for entry in WalkDir::new(source_root).into_iter() {
        let validated_entry = validate_dir_entry(entry, source_root)?;

        let entry_full_path = validated_entry.full_path;
        let entry_relative_path = validated_entry.relative_path;
        let entry_kind = validated_entry.kind;

        let destination_path = destination_root.join(&entry_relative_path);

        match entry_kind {
            EntryKind::Directory => {
                if entry_relative_path.as_os_str().is_empty() {
                    continue; // root itself
                }
                create_directories(&destination_path)?;
            }
            EntryKind::Symlink => {
                println!(
                    "WARNING: `{}` is a symlink. It is not supported by pom current version.",
                    entry_relative_path.display()
                );
                continue;
                // Doesn't justify an error
            }
            EntryKind::File => {
                if destination_path.exists() && existing_file_policy == ExistingFilePolicy::Fail {
                    return Err((
                        PomErrorCode::FilesystemFileOverwriteForbidden,
                        Some(format!("`{}` already exists.", destination_path.display())),
                    ));
                }

                if let Some(rendered_destination_path) =
                    get_rendered_template_dest(&destination_path)
                {
                    if let Some(extracted_fields) = fields {
                        copy_count += copy_with_rendering_helper(
                            &entry_full_path,
                            &rendered_destination_path,
                            extracted_fields,
                            &existing_file_policy,
                        )?;
                    }
                } else {
                    copy_count += plain_copy_helper(&entry_full_path, &destination_path)?;
                }
            }
        };
    }

    Ok(copy_count)
}

/// Helper for single-file copy logic.
fn plain_copy_helper(entry_path: &Path, destination_path: &Path) -> PomResult<usize> {
    match fs::copy(entry_path, destination_path) {
        Ok(_) => Ok(1),
        Err(src) => Err((
            PomErrorCode::FilesystemCopyFail,
            Some(format!("{}: {}", entry_path.display(), src)),
        )),
    }
}

/// Helper for single-file copy logic with rendering.
pub fn copy_with_rendering_helper(
    entry_path: &Path,
    destination_path: &Path,
    fields: &TemplateFields,
    existing_file_policy: &ExistingFilePolicy,
) -> PomResult<usize> {
    let template_content = read_file(entry_path)?;

    let rendered_content = render_template(&template_content, fields);

    write_file(destination_path, &rendered_content, existing_file_policy)?;

    Ok(1)
}

/// Read the content of a file.
///
/// # Errors
/// - `PomErrorCode::FilesystemReadTargetNotFound` if the target does not exist
/// - `PomErrorCode::FilesystemReadNotFile` if the target is not a file
/// - `PomErrorCode::FilesystemReadFailed` if readingfails due to an OS error
pub fn read_file(file_path: &Path) -> PomResult<String> {
    if !file_path.exists() {
        return Err((
            PomErrorCode::FilesystemReadTargetNotFound,
            Some(file_path.display().to_string()),
        ));
    }

    if !file_path.is_file() {
        return Err((
            PomErrorCode::FilesystemReadNotFile,
            Some(file_path.display().to_string()),
        ));
    }

    let file_content = fs::read_to_string(file_path)
        .map_err(|e| (PomErrorCode::FilesystemReadFailed, Some(e.to_string())))?;

    Ok(file_content)
}

/// Write a file.
///
/// The file is created if it doesn't already exist.
///
/// # Arguments
/// - `file_path` - Path to the file to write
/// - `file_content` - Content to write in the file
/// - `existing_file_policy` - What to do if a file already exists at the destination
///
/// # Errors
/// - `PomErrorCode::FilesystemFileOverwriteForbidden` if a file already exists and `existing_file_policy` is `ExistingFilePolicy::Fail`
/// - `PomErrorCode::FilesystemFileCreationFail` if creation fails due to an OS error
/// - `PomErrorCode::FilesystemFileWriteFail` if write fails due to an OS error
pub fn write_file(
    file_path: &Path,
    file_content: &str,
    existing_file_policy: &ExistingFilePolicy,
) -> PomResult<()> {
    if file_path.exists() && *existing_file_policy == ExistingFilePolicy::Fail {
        return Err((
            PomErrorCode::FilesystemFileOverwriteForbidden,
            Some(format!("`{}` already exists.", file_path.display())),
        ));
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true) // <-- THIS erases previous content
        .open(file_path)
        .map_err(|e| {
            (
                PomErrorCode::FilesystemFileCreationFail,
                Some(e.to_string()),
            )
        })?;

    file.write_all(file_content.as_bytes())
        .map_err(|e| (PomErrorCode::FilesystemFileWriteFail, Some(e.to_string())))?;

    Ok(())
}

/// Rename a file.
///
/// # Errors
/// - `PomErrorCode::FilesystemRenameOriginNotFound` if the target does not exist
/// - `PomErrorCode::FilesystemRenameDestinationExists` if the new name already exists
/// - `PomErrorCode::FilesystemRenameFailed` if rename fails due to an OS error
pub fn rename_file(old_path: &Path, new_path: &Path) -> PomResult<()> {
    if !old_path.exists() {
        return Err((
            PomErrorCode::FilesystemRenameOriginNotFound,
            Some(format!("{}", old_path.display())),
        ));
    }

    if new_path.exists() {
        return Err((
            PomErrorCode::FilesystemRenameDestinationExists,
            Some(format!("{}", new_path.display())),
        ));
    }
    fs::rename(old_path, new_path)
        .map_err(|e| (PomErrorCode::FilesystemRenameFailed, Some(e.to_string())))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::{TempDir, tempdir};

    #[cfg(test)]
    mod dir_creation_tests {
        use super::*;

        #[test]
        fn creates_dirs_when_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let project_root = tmp.path();

            let dirs = vec![
                project_root.join("src/app"),
                project_root.join("resources/doc"),
            ];

            create_directories(&dirs).unwrap();

            assert!(project_root.join("src").is_dir());
            assert!(project_root.join("src/app").is_dir());
            assert!(project_root.join("resources/doc").is_dir());
        }

        #[test]
        fn succeeds_if_dirs_already_exist() {
            let tmp = tempfile::tempdir().unwrap();
            let project_root = tmp.path();

            fs::create_dir_all(project_root.join("src/app")).unwrap();

            let dirs = vec![
                project_root.join("src/app"),
                project_root.join("resources/doc"),
            ];

            create_directories(&dirs).unwrap();

            assert!(project_root.join("src/app").is_dir());
            assert!(project_root.join("resources/doc").is_dir());
        }

        #[test]
        fn fails_if_dir_path_is_blocked_by_file() {
            let tmp = tempfile::tempdir().unwrap();
            let project_root = tmp.path();

            // Create a file "src" so "src/app" cannot become a directory
            let file_path = project_root.join("src");
            write_file(&file_path, "abc", &ExistingFilePolicy::Overwrite).unwrap();

            let dirs = vec![project_root.join("src/app")];

            let err = create_directories(&dirs).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemDirCreationFail);
        }
    }

    #[cfg(test)]
    mod file_copying_tests {
        use super::*;
        use crate::template_rendering::FieldKey;

        #[test]
        fn copy_files_and_file_tree() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            let src_root = src.path();
            let dst_root = dst.path();

            create_directories(&src_root.join("a/b")).unwrap();
            write_file(
                &src_root.join("root.txt"),
                "root",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();
            write_file(
                &src_root.join("a/file1.txt"),
                "one",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();
            write_file(
                &src_root.join("a/b/file2.txt"),
                "two",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();

            let n = copy_files(src_root, dst_root, ExistingFilePolicy::Overwrite, &None).unwrap();
            assert_eq!(n, 3);

            assert_eq!(read_file(&dst_root.join("root.txt")).unwrap(), "root");
            assert_eq!(read_file(&dst_root.join("a/file1.txt")).unwrap(), "one");
            assert_eq!(read_file(&dst_root.join("a/b/file2.txt")).unwrap(), "two");

            assert!(dst_root.join("a/b").is_dir());
        }

        #[test]
        fn templates_copied_with_rendering() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            let src_root = src.path();
            let dst_root = dst.path();

            write_file(
                &src_root.join("normal_file.txt"),
                "A file without fields",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();
            write_file(
                &src_root.join("template_file.txt.pomrt"),
                "A template file with an arbitrary field {{pom:module_doxygen_group}}.",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();

            let mut template_fields = TemplateFields::new();
            template_fields.insert(FieldKey::ModuleDoxygenGroup, "rendered");

            let n = copy_files(
                src_root,
                dst_root,
                ExistingFilePolicy::Overwrite,
                &Some(template_fields),
            )
            .unwrap();

            assert_eq!(n, 2);
            assert_eq!(
                read_file(&dst_root.join("normal_file.txt")).unwrap(),
                "A file without fields"
            );
            assert_eq!(
                read_file(&dst_root.join("template_file.txt")).unwrap(),
                "A template file with an arbitrary field rendered."
            );
        }

        #[test]
        fn fails_if_source_missing() {
            let dst = tempdir().unwrap();
            let err = copy_files(
                std::path::Path::new("this-path-should-not-exist-___"),
                dst.path(),
                ExistingFilePolicy::Overwrite,
                &None,
            )
            .unwrap_err();

            assert_eq!(err.0, PomErrorCode::FilesystemCopySourceMissing);
        }

        #[test]
        fn fails_if_source_not_dir() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            let file = src.path().join("not_a_dir.txt");
            fs::write(&file, "x").unwrap();

            let err =
                copy_files(&file, dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemCopySourceNotDir);
        }

        #[test]
        fn fails_on_existing_file_when_policy_fail() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            fs::write(src.path().join("a.txt"), "SRC").unwrap();

            // Pre-create destination file with same relative path
            fs::write(dst.path().join("a.txt"), "DST").unwrap();

            let err =
                copy_files(src.path(), dst.path(), ExistingFilePolicy::Fail, &None).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemFileOverwriteForbidden);
        }

        #[test]
        fn overwrites_existing_file_when_policy_allows() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            fs::write(src.path().join("a.txt"), "NEW").unwrap();
            fs::write(dst.path().join("a.txt"), "OLD").unwrap();

            let n =
                copy_files(src.path(), dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap();
            assert_eq!(n, 1);

            let content = read_file(&dst.path().join("a.txt")).unwrap();
            assert_eq!(content, "NEW");
        }

        #[test]
        fn skips_symlinks() {
            use std::os::unix::fs::symlink;

            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            fs::write(src.path().join("real.txt"), "REAL").unwrap();
            symlink(src.path().join("real.txt"), src.path().join("link.txt")).unwrap();

            let n =
                copy_files(src.path(), dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap();

            // Only real.txt copied; link.txt should be skipped
            assert_eq!(n, 1);
            assert!(dst.path().join("real.txt").exists());
            assert!(!dst.path().join("link.txt").exists());
        }
    }

    #[cfg(test)]
    mod file_reading_tests {
        use super::*;

        fn create_temp_file() -> (TempDir, PathBuf) {
            let temp_dir = tempdir().unwrap();
            let file_path = temp_dir.path().join("hello.txt");

            write_file(&file_path, "Hello, World!", &ExistingFilePolicy::Overwrite).unwrap();

            (temp_dir, file_path)
        }

        #[test]
        fn read_existing_file() {
            let (_temp_dir, file_path) = create_temp_file();

            let file_content = read_file(&file_path).unwrap();

            assert_eq!(file_content, "Hello, World!");
        }

        #[test]
        fn reject_not_found_target() {
            let temp_dir = tempdir().unwrap();
            let missing_file = temp_dir.path().join("missing.txt");

            let err = read_file(&missing_file).unwrap_err();

            assert_eq!(err.0, PomErrorCode::FilesystemReadTargetNotFound);
        }

        #[test]
        fn reject_directory_target() {
            let temp_dir = tempdir().unwrap();
            let dir_path = temp_dir.path().join("my_dir");

            fs::create_dir(&dir_path).unwrap();

            let err = read_file(&dir_path).unwrap_err();

            assert_eq!(err.0, PomErrorCode::FilesystemReadNotFile);
        }
    }

    #[cfg(test)]
    mod file_writing_tests {
        use super::*;

        #[test]
        fn creates_file_and_writes_content() {
            let dir = tempdir().unwrap();
            let p = dir.path().join("x.txt");

            write_file(&p, "hello", &ExistingFilePolicy::Overwrite).unwrap();
            assert_eq!(read_file(&p).unwrap(), "hello");
        }

        #[test]
        fn overwrites_and_truncates() {
            let dir = tempdir().unwrap();
            let p = dir.path().join("x.txt");

            fs::write(&p, "0123456789").unwrap();
            write_file(&p, "abc", &ExistingFilePolicy::Overwrite).unwrap();

            // truncate(true) should have removed old tail
            assert_eq!(read_file(&p).unwrap(), "abc");
        }

        #[test]
        fn fails_if_exists_and_policy_fail() {
            let dir = tempdir().unwrap();
            let p = dir.path().join("x.txt");

            fs::write(&p, "existing").unwrap();
            let err = write_file(&p, "new", &ExistingFilePolicy::Fail).unwrap_err();

            assert_eq!(err.0, PomErrorCode::FilesystemFileOverwriteForbidden);
        }
    }

    #[cfg(test)]
    mod file_rename_tests {
        use super::*;

        fn create_temp_files() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
            let temp_dir = tempdir().unwrap();
            let test_root = temp_dir.path().join("project_root");

            create_directories(&test_root).unwrap();

            let file_hello_world = test_root.join("hello_world.txt");
            write_file(
                &file_hello_world,
                "Hello, World!",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();

            let file_hello_rust = test_root.join("hello_rust.txt");
            write_file(
                &file_hello_rust,
                "Hello, Rust!",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();

            return (temp_dir, test_root, file_hello_world, file_hello_rust);
        }

        #[test]
        fn reject_not_found_files() {
            let (_temp_dir, test_root, _, file_hello_rust) = create_temp_files();

            let wrong_file = test_root.join("wrong_file.txt");

            let err = rename_file(&wrong_file, &file_hello_rust).unwrap_err();

            assert_eq!(err.0, PomErrorCode::FilesystemRenameOriginNotFound);
        }

        #[test]
        fn reject_existing_destination() {
            let (_temp_dir, _, file_hello_world, file_hello_rust) = create_temp_files();
            let err = rename_file(&file_hello_world, &file_hello_rust).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemRenameDestinationExists);
        }

        #[test]
        fn rename_file_renames_file() {
            let (_temp_dir, _test_root, file_hello_world, _) = create_temp_files();

            let new_path = file_hello_world.with_file_name("hello_universe.txt");

            rename_file(&file_hello_world, &new_path).unwrap();

            assert!(!file_hello_world.exists());
            assert!(new_path.exists());

            let renamed_file_content = fs::read_to_string(&new_path).unwrap();

            assert_eq!(renamed_file_content, String::from("Hello, World!"));
        }
    }
}

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
/// - `PomErrorCode::FilesystemDirCreationFail` if a directory creation failed due to an OS error
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
/// - `PomErrorCode::FilesystemReadFailed` if reading failed due to an OS error
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

    match file.write_all(file_content.as_bytes()) {
        Ok(()) => Ok(()),
        Err(src) => Err((PomErrorCode::FilesystemFileWriteFail, Some(src.to_string()))),
    }
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

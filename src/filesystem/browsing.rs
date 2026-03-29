//! Filesystem browsing utilities.
//!
//! Provides helpers to traverse and validate filesystem entries.

use crate::errors::{PomErrorCode, PomResult};
use std::path::{Path, PathBuf};
use walkdir;

#[derive(Debug, PartialEq)]
/// Reperesents the type of a `WalkDir::EntryDir`
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug)]
/// Represents the data extracted from a `WalkDir::EntryDir` after validation
pub struct ValidatedEntry {
    pub full_path: PathBuf,
    pub relative_path: PathBuf,
    pub kind: EntryKind,
}

/// Check if an entry gave by `WalkDir::new().into_iter()` is valid
///
/// # Arguments
/// - `entry` - The entry to analyze
///
/// # Returns
/// The useful data extracted from the entry. See `ValidatedEntry` for details.
///
/// # Errors
/// - `PomErrorCode::FilesystemEntryInvalid` if the entry is invalid due to an OS reason
/// - `PomErrorCode::FilesystemStripPathPrefixFail` if relative path extraction failed
/// - `PomErrorCode::FilesystemUnsupportedEntryType` if the entry type is not supported by Pom
pub fn validate_dir_entry(
    entry: Result<walkdir::DirEntry, walkdir::Error>,
    project_root: &Path,
) -> PomResult<ValidatedEntry> {
    let entry = match entry {
        Ok(entry) => entry,
        Err(src) => {
            return Err((PomErrorCode::FilesystemEntryInvalid, Some(src.to_string())));
        }
    };

    let full_path = entry.path();

    let relative_path = match full_path.strip_prefix(project_root) {
        Ok(relative_path) => relative_path,
        Err(src) => {
            return Err((
                PomErrorCode::FilesystemStripPathPrefixFail,
                Some(src.to_string()),
            ));
        }
    };

    let file_type = entry.file_type();
    let kind = if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_symlink() {
        EntryKind::Symlink
    } else {
        return Err((
            PomErrorCode::FilesystemUnsupportedEntryType,
            Some(full_path.display().to_string()),
        ));
    };

    Ok(ValidatedEntry {
        full_path: full_path.to_path_buf(),
        relative_path: relative_path.to_path_buf(),
        kind,
    })
}

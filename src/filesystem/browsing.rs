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
/// - `PomErrorCode::FilesystemStripPathPrefixFail` if relative path extractionfails
/// - `PomErrorCode::FilesystemUnsupportedEntryType` if the entry type is not supported by Pom
pub fn validate_dir_entry(
    entry: Result<walkdir::DirEntry, walkdir::Error>,
    project_root: &Path,
) -> PomResult<ValidatedEntry> {
    let entry = entry.map_err(|e| (PomErrorCode::FilesystemEntryInvalid, Some(e.to_string())))?;

    let full_path = entry.path();

    let relative_path = full_path.strip_prefix(project_root).map_err(|e| {
        (
            PomErrorCode::FilesystemStripPathPrefixFail,
            Some(e.to_string()),
        )
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::io_ops::{ExistingFilePolicy, create_directories, write_file};
    use tempfile::{TempDir, tempdir};

    mod dir_entry_validation_tests {
        use super::*;

        fn test_temp_root_creation_helper() -> (TempDir, PathBuf) {
            let temp_dir = tempdir().unwrap();
            let src_root = temp_dir.path().join("src_root");
            test_dir_creation_helper(&src_root);
            (temp_dir, src_root.to_path_buf())
        }

        fn test_dir_creation_helper(dir_path: &Path) {
            create_directories(dir_path).unwrap();
        }

        fn test_file_creation_helper(file_path: &Path) {
            write_file(
                file_path,
                "Some file content",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();
        }

        fn test_entry_extraction_helper(
            root: &Path,
            target: &Path,
        ) -> walkdir::Result<walkdir::DirEntry> {
            walkdir::WalkDir::new(root)
                .into_iter()
                .find_map(|entry| match entry {
                    Ok(de) if de.path() == target => Some(Ok(de)),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                })
                .expect("expected to find the file entry")
        }

        #[test]
        fn extract_useful_data_from_file_entry() {
            let (_temp_dir, src_root) = test_temp_root_creation_helper();
            let dir_path_relative = PathBuf::from("a/b");
            let dir_path_absolute = src_root.join(&dir_path_relative);
            let file_path_relative = dir_path_relative.join("hello.txt");
            let file_path_absolute = src_root.join(&file_path_relative);
            test_dir_creation_helper(&dir_path_absolute);
            test_file_creation_helper(&file_path_absolute);

            let entry = test_entry_extraction_helper(&src_root, &file_path_absolute);
            let validated = validate_dir_entry(entry, &src_root).unwrap();

            assert_eq!(validated.full_path, file_path_absolute);
            assert_eq!(validated.relative_path, file_path_relative);
            assert!(matches!(validated.kind, EntryKind::File));
        }

        #[test]
        fn extract_useful_data_from_dir_entry() {
            let (_temp_dir, src_root) = test_temp_root_creation_helper();
            let dir_path_relative = PathBuf::from("a/b");
            let dir_path_absolute = src_root.join(&dir_path_relative);
            test_dir_creation_helper(&dir_path_absolute);

            let entry = test_entry_extraction_helper(&src_root, &dir_path_absolute);
            let validated = validate_dir_entry(entry, &src_root).unwrap();

            assert_eq!(validated.full_path, dir_path_absolute);
            assert_eq!(validated.relative_path, dir_path_relative);
            assert!(matches!(validated.kind, EntryKind::Directory));
        }

        #[test]
        fn catch_invalid_entry() {
            let (_temp_dir, src_root) = test_temp_root_creation_helper();

            // Force a WalkDir error by iterating a non-existent path.
            let mut it = walkdir::WalkDir::new(src_root.join("does-not-exist")).into_iter();
            let first = it.next().expect("expected an item (Err) from iterator");

            let err = validate_dir_entry(first, &src_root).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemEntryInvalid);
            assert!(err.1.is_some());
        }

        #[test]
        fn catch_prefix_strip_fails() {
            let (_temp_dir, src_root) = test_temp_root_creation_helper();
            let file_path_relative = PathBuf::from("hello.txt");
            let file_path_absolute = src_root.join(&file_path_relative);
            test_file_creation_helper(&file_path_absolute);

            let entry = test_entry_extraction_helper(&src_root, &file_path_absolute);

            // Pass a different root so strip_prefix fails
            let (_temp_dir_2, src_root_2) = test_temp_root_creation_helper();

            let err = validate_dir_entry(entry, &src_root_2).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemStripPathPrefixFail);
        }

        #[test]
        fn validate_dir_entry_unsupported_entry_type_fifo() {
            use std::ffi::CString;

            let (_temp_dir, src_root) = test_temp_root_creation_helper();

            let fifo_path = src_root.join("myfifo");
            let cpath = CString::new(fifo_path.to_string_lossy().as_bytes()).unwrap();
            let rc = unsafe { libc::mkfifo(cpath.as_ptr(), 0o644) };
            assert_eq!(rc, 0, "mkfifo failed");

            let entry = test_entry_extraction_helper(&src_root, &fifo_path);

            let err = validate_dir_entry(entry, &src_root).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemUnsupportedEntryType);
            assert!(err.1.unwrap().contains("myfifo"));
        }
    }
}

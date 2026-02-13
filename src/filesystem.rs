//! Filesystem module
//!
//! This module is responsible for wraping interaction with the system OS Filesystem
//! API (`std::fs`). It handles verification and error interpretation for file copy, creation, for directory walking...

use std::path::{PathBuf, Path};
use crate::errors::{PomErrorCode, PomResult};
use std::fs::{self, OpenOptions};
use std::io::Write;
use walkdir::WalkDir;
use crate::template_rendering::{get_rendered_template_dest, render_template, TemplateFields};



pub mod path_list {
    //! path_list module
    //!
    //! This module defines the trait `PathList`, to handle some functions to accept a
    //! sole Path / PathBuf or a collection.

    use std::path::{PathBuf, Path};


    pub trait PathList {
        // Learning notes: We define a trait = a list of methods a type must provide
        // to be considered a `PathList`.
        // Any type implementing `PathList` must provide this method.

        /// Return an iterator of references on the provided element or on the provided collection elements
        fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_>;
        // Any type implementing `PathList` must provide this method.
        //
        // Return type dissection:
        // - `Iterator` is a *trait* (not a concrete type). It has an associated type `Item`.
        // - `Iterator<Item = &Path>` means: "an iterator whose yielded items are `&Path`".
        // - `dyn Iterator<...>` means: "a *trait object*": the concrete iterator type is
        //   intentionally hidden/unknown to the caller; calls go through dynamic dispatch
        //   (vtable). This lets us return different concrete iterator types from the same
        //   function (e.g. `once(...)` vs `slice.iter().map(...)`).
        // - `Box<dyn ...>`: a `dyn Trait` value has unknown size at compile time, so it must
        //   live behind a pointer. `Box` is an owning heap pointer, suitable for returning
        //   an iterator created inside this method.
        // - `+ '_`: this ties the trait object's lifetime to `&self`. The iterator may borrow
        //   from `self` (it yields `&Path` that come from paths stored in `self`), therefore
        //   the iterator cannot outlive `self`. Equivalent explicit form:
        //       `fn iter_paths<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Path> + 'a>`
        //
        // TL;DR: "Return an owned (Boxed) trait object iterator over borrowed `&Path` items,
        //         and ensure it can't outlive `self`."
    }


    impl PathList for Path {
        fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
            Box::new(std::iter::once(self))
        }
    }


    impl PathList for PathBuf {
        fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
            Box::new(  // Placed on the heap so it can be returned by reference
                std::iter::once(  // An iterator that yields exactly one element
                    self.as_path() // A borrowed view of `self`
                )
            )
        }
    }


    impl PathList for [PathBuf] {
        fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
            Box::new(
                self.iter()  // `self` is `[PathBuf]`, we take an iterator over references to its elements
                .map(|p| p.as_path()))  // Transforms ("maps") each item (`|p|`) of this iterator into something else. Here: `&Path`s
        }
    }


    impl PathList for Vec<PathBuf> {
        fn iter_paths(&self) -> Box<dyn Iterator<Item = &Path> + '_> {
            self.as_slice().iter_paths()
        }
    }

}


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


/// Represents the action to take when a file write or copy conflicts with an existing file
#[derive(PartialEq)]
pub enum ExistingFilePolicy {
    // Overwrite,  // Cancel "unused" warning for now - make it available during unit tests
    Fail,
}


/// Create directories passed by reference
///
/// Existing directories are ignored
/// Parents are created as needed
/// May error `PomErrorCode::FilesystemDirCreationFail`
pub fn create_directories(dirs_paths: &(impl path_list::PathList + ?Sized)) -> PomResult<()> {
    // Read parameter type as "A reference to a type implementing `PathList`"
    for dir_path in dirs_paths.iter_paths() {
        match fs::create_dir_all(dir_path) {
            Ok(()) => { continue },
            Err(src) => {
                let details = format!("{}: {:?}",dir_path.display(), src);
                return Err((
                    PomErrorCode::FilesystemDirCreationFail,
                    Some(details),
                ));
            }
        };
    }
    Ok(())
}


/// Check if an entry gave by `WalkDir::new().into_iter()` is valid
///
/// May error `PomErrorCode::FilesystemStripPathPrefixFail`
pub fn validate_dir_entry(entry: Result<walkdir::DirEntry, walkdir::Error>, source_root: &Path) -> PomResult<ValidatedEntry> {
    let entry = match entry {
        Ok(entry) => entry,
        Err(src) => {
            return Err((
                PomErrorCode::FilesystemEntryInvalid,
                Some(src.to_string()),
            ));
        },
    };

    let full_path = entry.path();

    let relative_path = match full_path.strip_prefix(source_root) {
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
        kind: kind,
    })
}


/// Copy files contained in a directory from a source to a destination
///
/// Create required sub directories to respect the source file tree
/// May error `PomErrorCode::FilesystemFileOverwriteForbidded` or `FilesystemCopyFail`
pub fn copy_files(source_root: &Path, destination_root: &Path, existing_file_policy: ExistingFilePolicy, fields: &Option<TemplateFields>) -> PomResult<usize> {
    if !source_root.exists() {
        return Err((
            PomErrorCode::FilesystemCopySourceMissing,
            Some(source_root.display().to_string())
        ));
    }
    if !source_root.is_dir() {
        return Err((
            PomErrorCode::FilesystemCopySourceNotDir,
            Some(source_root.display().to_string())
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
            },
            EntryKind::Symlink => {
                println!("WARNING: `{}` is a symlink. It is not supported by pom current version.", entry_relative_path.display());
                continue;
                // Doesn't justify an error
            },
            EntryKind::File => {
                if destination_path.exists() {
                    if existing_file_policy == ExistingFilePolicy::Fail {
                        return Err((
                            PomErrorCode::FilesystemFileOverwriteForbidded,
                            Some(format!("`{}` already exists.", destination_path.display())),
                        ));

                    }
                }

                if let Some(rendered_destination_path) = get_rendered_template_dest(&destination_path){
                    if let Some(extracted_fields) = fields {
                        copy_count+= copy_with_rendering_helper(&entry_full_path,
                                                                &rendered_destination_path,
                                                                extracted_fields,
                                                                &existing_file_policy
                                                                )?;
                    }
                } else {
                    copy_count += plain_copy_helper(&entry_full_path, &destination_path)?;
                }
            },
        };
    }

    Ok(copy_count)
}


fn plain_copy_helper(entry_path: &Path, destination_path: &Path) -> PomResult<usize> {
    match fs::copy(&entry_path, &destination_path) {
        Ok(_) => { return Ok(1); }
        Err(src) => {
            return Err((
                PomErrorCode::FilesystemCopyFail,
                Some(format!("{}: {}", entry_path.display(), src)),
            ));
        }
    }
}


fn copy_with_rendering_helper(
    entry_path: &Path,
    destination_path: &Path,
    fields: &TemplateFields,
    existing_file_policy: &ExistingFilePolicy
) -> PomResult<usize> {
    let template_content = match std::fs::read_to_string(entry_path) {
        Ok(template_content) => template_content,
        Err(src) => { return Err((
            PomErrorCode::FilesystemFileReadFail,
            Some(format!("{}: {}", entry_path.display(), src))
        ));},
    };

    let rendered_content = render_template(&template_content, fields);

    write_file(destination_path, &rendered_content, existing_file_policy)?;

    Ok(1)
}


/// Write a file
///
/// Override if it exists
/// May error `PomErrorCode::FilesystemFileCreationFail`, `FilesystemFileWriteFail` or `FilesystemFileOverwriteForbidded`
pub fn write_file(file_path: &Path, file_content: &str, existing_file_policy: &ExistingFilePolicy)  -> PomResult<()>  {
    if file_path.exists() {
        if *existing_file_policy == ExistingFilePolicy::Fail {
            return Err((
                PomErrorCode::FilesystemFileOverwriteForbidded,
                Some(format!("`{}` already exists.", file_path.display())),
            ));

        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true) // <-- THIS erases previous content
        .open(&file_path)
        .map_err(|e| (PomErrorCode::FilesystemFileCreationFail, Some(e.to_string())))?;

    match file.write_all(file_content.as_bytes()){
        Ok(()) => { Ok(()) }
        Err(src) => {
            Err((
                PomErrorCode::FilesystemFileWriteFail,
                Some(src.to_string()),
            ))
        }
    }
}



#[cfg(test)]
mod tests{
    use super::*;
    use tempfile::tempdir;
    use std::io::Read;

    mod subdirs_creation {
        use super::*;
        use std::fs::File;

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
            File::create(project_root.join("src")).unwrap();

            let dirs = vec![
                project_root.join("src/app"),
            ];

            let err = create_directories(&dirs).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemDirCreationFail);
        }
    }

    mod validate_dir_entry_tests {
        use super::*;

        #[test]
        fn validate_dir_entry_file_ok_relative_path_and_kind() {
            let src = tempdir().unwrap();
            let src_root = src.path();

            let nested = src_root.join("a/b");
            fs::create_dir_all(&nested).unwrap();

            let file_path = nested.join("hello.txt");
            fs::write(&file_path, b"hi").unwrap();

            // WalkDir yields the root first, then children. We'll find our file entry.
            let entry = walkdir::WalkDir::new(src_root)
                .into_iter()
                .find_map(|e| match e {
                    Ok(de) if de.path() == file_path => Some(Ok(de)),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                })
                .expect("expected to find the file entry");

            let validated = validate_dir_entry(entry, src_root).unwrap();

            assert_eq!(validated.full_path, file_path);
            assert_eq!(validated.relative_path, std::path::PathBuf::from("a/b/hello.txt"));
            assert!(matches!(validated.kind, EntryKind::File));
        }

        #[test]
        fn validate_dir_entry_dir_ok_relative_path_and_kind() {
            let src = tempdir().unwrap();
            let src_root = src.path();

            let nested = src_root.join("dir1/dir2");
            fs::create_dir_all(&nested).unwrap();

            let entry = walkdir::WalkDir::new(src_root)
                .into_iter()
                .find_map(|e| match e {
                    Ok(de) if de.path() == nested => Some(Ok(de)),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                })
                .expect("expected to find the dir entry");

            let validated = validate_dir_entry(entry, src_root).unwrap();

            assert_eq!(validated.full_path, nested);
            assert_eq!(validated.relative_path, std::path::PathBuf::from("dir1/dir2"));
            assert!(matches!(validated.kind, EntryKind::Directory));
        }

        #[test]
        fn validate_dir_entry_err_is_mapped() {
            let src = tempdir().unwrap();
            let src_root = src.path();

            // Force a WalkDir error by iterating a non-existent path.
            let mut it = walkdir::WalkDir::new(src_root.join("does-not-exist")).into_iter();
            let first = it.next().expect("expected an item (Err) from iterator");

            let err = validate_dir_entry(first, src_root).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemEntryInvalid);
            assert!(err.1.is_some());
        }

        #[test]
        fn validate_dir_entry_strip_prefix_fail_is_mapped() {
            let src = tempdir().unwrap();
            let src_root = src.path();

            let file_path = src_root.join("x.txt");
            fs::write(&file_path, b"x").unwrap();

            let entry = walkdir::WalkDir::new(src_root)
                .into_iter()
                .find_map(|e| match e {
                    Ok(de) if de.path() == file_path => Some(Ok(de)),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                })
                .expect("expected to find file entry");

            // Pass a different root so strip_prefix fails
            let other_root = tempdir().unwrap();

            let err = validate_dir_entry(entry, other_root.path()).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemStripPathPrefixFail);
            assert!(err.1.is_some());
        }

        #[test]
        fn validate_dir_entry_unsupported_entry_type_fifo() {
            use std::ffi::CString;

            let src = tempdir().unwrap();
            let src_root = src.path();

            let fifo_path = src_root.join("myfifo");
            let cpath = CString::new(fifo_path.to_string_lossy().as_bytes()).unwrap();
            let rc = unsafe { libc::mkfifo(cpath.as_ptr(), 0o644) };
            assert_eq!(rc, 0, "mkfifo failed");

            let entry = walkdir::WalkDir::new(src_root)
                .into_iter()
                .find_map(|e| match e {
                    Ok(de) if de.path() == fifo_path => Some(Ok(de)),
                    Ok(_) => None,
                    Err(err) => Some(Err(err)),
                })
                .expect("expected to find fifo entry");

            let err = validate_dir_entry(entry, src_root).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemUnsupportedEntryType);
            assert!(err.1.unwrap().contains("myfifo"));
        }

    }

    mod copy_files_tests {
        use super::*;

        fn read_to_string(p: &std::path::Path) -> String {
            let mut s = String::new();
            fs::File::open(p).unwrap().read_to_string(&mut s).unwrap();
            s
        }

        #[test]
        fn copy_files_copies_tree_and_counts_files() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            let src_root = src.path();
            let dst_root = dst.path();

            fs::create_dir_all(src_root.join("a/b")).unwrap();
            fs::write(src_root.join("root.txt"), "root").unwrap();
            fs::write(src_root.join("a/file1.txt"), "one").unwrap();
            fs::write(src_root.join("a/b/file2.txt"), "two").unwrap();

            let n = copy_files(src_root, dst_root, ExistingFilePolicy::Overwrite, &None).unwrap();
            assert_eq!(n, 3);

            assert_eq!(read_to_string(&dst_root.join("root.txt")), "root");
            assert_eq!(read_to_string(&dst_root.join("a/file1.txt")), "one");
            assert_eq!(read_to_string(&dst_root.join("a/b/file2.txt")), "two");
            assert!(dst_root.join("a/b").is_dir());
        }

        #[test]
        fn copy_files_fails_if_source_missing() {
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
        fn copy_files_fails_if_source_not_dir() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            let file = src.path().join("not_a_dir.txt");
            fs::write(&file, "x").unwrap();

            let err = copy_files(&file, dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemCopySourceNotDir);
        }

        #[test]
        fn copy_files_fails_on_existing_file_when_policy_fail() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            fs::write(src.path().join("a.txt"), "SRC").unwrap();

            // Pre-create destination file with same relative path
            fs::write(dst.path().join("a.txt"), "DST").unwrap();

            let err = copy_files(src.path(), dst.path(), ExistingFilePolicy::Fail, &None).unwrap_err();
            assert_eq!(err.0, PomErrorCode::FilesystemFileOverwriteForbidded);
        }

        #[test]
        fn copy_files_overwrites_existing_file_when_policy_allows() {
            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            fs::write(src.path().join("a.txt"), "NEW").unwrap();
            fs::write(dst.path().join("a.txt"), "OLD").unwrap();

            let n = copy_files(src.path(), dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap();
            assert_eq!(n, 1);

            let content = read_to_string(&dst.path().join("a.txt"));
            assert_eq!(content, "NEW");
        }

        #[test]
        fn copy_files_skips_symlinks() {
            use std::os::unix::fs::symlink;

            let src = tempdir().unwrap();
            let dst = tempdir().unwrap();

            fs::write(src.path().join("real.txt"), "REAL").unwrap();
            symlink(src.path().join("real.txt"), src.path().join("link.txt")).unwrap();

            let n = copy_files(src.path(), dst.path(), ExistingFilePolicy::Overwrite, &None).unwrap();

            // Only real.txt copied; link.txt should be skipped
            assert_eq!(n, 1);
            assert!(dst.path().join("real.txt").exists());
            assert!(!dst.path().join("link.txt").exists());
        }
    }

    mod write_file_tests {
        use super::*;

        fn read_to_string(p: &std::path::Path) -> String {
            let mut s = String::new();
            fs::File::open(p).unwrap().read_to_string(&mut s).unwrap();
            s
        }

        #[test]
        fn write_file_creates_and_writes() {
            let dir = tempdir().unwrap();
            let p = dir.path().join("x.txt");

            write_file(&p, "hello", &ExistingFilePolicy::Overwrite).unwrap();
            assert_eq!(read_to_string(&p), "hello");
        }

        #[test]
        fn write_file_overwrites_and_truncates() {
            let dir = tempdir().unwrap();
            let p = dir.path().join("x.txt");

            fs::write(&p, "0123456789").unwrap();
            write_file(&p, "abc", &ExistingFilePolicy::Overwrite).unwrap();

            // truncate(true) should have removed old tail
            assert_eq!(read_to_string(&p), "abc");
        }

        #[test]
        fn write_file_fails_if_exists_and_policy_fail() {
            let dir = tempdir().unwrap();
            let p = dir.path().join("x.txt");

            fs::write(&p, "existing").unwrap();
            let err = write_file(&p, "new", &ExistingFilePolicy::Fail).unwrap_err();

            assert_eq!(err.0, PomErrorCode::FilesystemFileOverwriteForbidded);
        }
    }
}

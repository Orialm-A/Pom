//! Filesystem module
//!
//! This module is responsible for wraping interaction with the system OS Filesystem
//! API (`std::fs`). It handles verification and error interpretation for file copy, creation, for directory walking...

use std::path::{PathBuf, Path};
use crate::errors::{PomErrorCode, PomResult};
use std::fs;
use walkdir::WalkDir;


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


/// Represents the data extracted from a `WalkDir::EntryDir` after validation
pub struct ValidatedEntry {
    pub full_path: PathBuf,
    pub relative_path: PathBuf,
    pub kind: EntryKind,
}


/// Create directories passed by reference
///
/// Existing directories are ignored
/// Parents are created as needed
/// May return `PomErrorCode::FilesystemDirCreationFail`
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
/// May return `PomErrorCode::FilesystemStripPathPrefixFail`
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
/// May return `PomErrorCode::FilesystemCopyDestExists` or `FilesystemCopyDestExists`
pub fn copy_files(source_root: &Path, destination_root: &Path) -> PomResult<usize> {
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
                    return Err((
                        PomErrorCode::FilesystemCopyDestExists,
                        Some(format!("`{}` already exists.", destination_path.display())),
                    ));
                }

                match fs::copy(&entry_full_path, &destination_path) {
                    Ok(_) => copy_count += 1,
                    Err(src) => {
                        return Err((
                            PomErrorCode::FilesystemCopyFail,
                            Some(format!("{}: {}", entry_full_path.display(), src)),
                        ));
                    }
                }
            },
        };
    }

    Ok(copy_count)
}

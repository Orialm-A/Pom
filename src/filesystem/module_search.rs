//! Module discovery within a Pom project.
//!
//! Provides utilities to search for C modules in the project filesystem.


use std::path::{Path, PathBuf};
use crate::errors::{PomErrorCode, PomResult};
use std::collections::{HashMap, HashSet};
use walkdir::WalkDir;
use crate::filesystem::browsing::{EntryKind, validate_dir_entry};


/// Describe which files were found for a module in a given directory.
///
/// A module may contain:
/// - A header file only (`<name>.h`)
/// - A source file only (`<name>.c`)
/// - Both
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ModuleFiles {
    pub header_file: bool,
    pub source_file: bool,
}

/// Store all modules location found while searching in a Pom project.
///
/// The key is the directory containing the module files, relative to the
/// project root. The value indicates whether a header file and/or a source
/// file was found there for the searched module name.
#[derive(Debug, Eq, PartialEq)]
pub struct ModulesFound {
    modules: HashMap<PathBuf, ModuleFiles>,
}

impl ModulesFound {
    /// Create an empty search result.
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }



    pub fn len(&self) -> usize {
        self.modules.len()
    }



    /// Mark that a header file was found for the module in `module_path`.
    ///
    /// If this module location was not registered yet, it is inserted first.
    /// This function is idempotent.
    ///
    /// # Arguments
    /// - `module_path` - Location of the module for which a header file has been found
    pub fn insert_header(&mut self, module_path: &Path) {
        self.insert_module(module_path);
        if let Some(reference) = self.modules.get_mut(module_path) {
            reference.header_file = true;
        }
    }

    /// Mark that a source file was found for the module in `module_path`.
    ///
    /// If this module location was not registered yet, it is inserted first.
    /// This function is idempotent.
    ///
    /// # Arguments
    /// - `module_path` - Location of the module for which a source file has been found
    pub fn insert_source(&mut self, module_path: &Path) {
        self.insert_module(module_path);
        if let Some(reference) = self.modules.get_mut(module_path) {
            reference.source_file = true;
        }
    }

    /// Insert a module location in the collection if it is not already present.
    ///
    /// Newly inserted entries are initialized with both file flags set to `false`.
    /// This function is idempotent.
    ///
    /// # Arguments
    /// - `module_path` - Module location
    fn insert_module(&mut self, module_path: &Path) {
        if !self.modules.contains_key(module_path) {
            let new_module = ModuleFiles {
                header_file: false,
                source_file: false,
            };
            self.modules.insert(module_path.to_path_buf(), new_module);
        }
    }



    /// Return all the locations found as `Vec<String>`
    ///
    /// This is used for menu selection.
    pub fn get_all_locations_as_text(&self) -> Vec<String> {
        self.modules
            .keys()
            .map(|path| path.display().to_string())
            .collect()
    }

    /// Return the module location if exactly one module is found.
    ///
    /// # Errors
    /// - `PomErrorCode::FileSystemModuleSearchResultNotUnique` if zero or multiple modules are found
    pub fn get_unique_location(&self) -> PomResult<PathBuf> {
        if self.modules.len() != 1 {
            return Err((PomErrorCode::FileSystemModuleSearchResultNotUnique, None));
        }

        Ok(self.modules.keys().next().unwrap().clone())
    }



    /// Return the file presence information for a module at the given location.
    ///
    /// # Arguments
    /// - `module_location` - Path to the module to query
    ///
    /// # Errors
    /// - `PomErrorCode::FileSystemModuleSearchResultKeyNotFound` if the
    /// provided `module_location` does not exist in the search results
    pub fn get_module_files(&self, module_location: &Path) -> PomResult<ModuleFiles> {
        if let Some(module) = self.modules.get(module_location) {
            Ok(module.clone())
        } else {
            Err((PomErrorCode::FileSystemModuleSearchResultKeyNotFound, None))
        }
    }
}



/// Search for modules with the given name within the project.
///
/// The search is performed recursively in each directory of `search_scope`.
///
/// # Arguments
/// - `module_name` - Name of the module to search for
/// - `project_root` - Path to the project root
/// - `search_scope` - Directories to explore recursively
///
/// # Errors
/// - Propagates filesystem-related error encountered during execution
pub fn search_module(
    module_name: &str,
    project_root: &Path,
    search_scope: &HashSet<PathBuf>,
) -> PomResult<ModulesFound> {
    let header_name = format!("{}.h", &module_name);
    let source_name = format!("{}.c", &module_name);

    let mut found = ModulesFound::new();

    for dir in search_scope {
        let search_path = project_root.join(dir);
        for entry in WalkDir::new(search_path).into_iter() {
            let validated_entry = validate_dir_entry(entry, project_root)?;

            let entry_relative_path = validated_entry.relative_path;
            let entry_kind = validated_entry.kind;

            match entry_kind {
                EntryKind::Directory => {
                    continue;
                    // Sources may contain nested directories, we just want files
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
                    let file_name = entry_relative_path.file_name();
                    let module_location = entry_relative_path.parent().unwrap();

                    if let Some(file_name) = file_name {
                        if file_name == header_name.as_str() {
                            found.insert_header(module_location);
                        }
                        if file_name == source_name.as_str() {
                            found.insert_source(module_location);
                        }
                    }
                }
            }
        }
    }
    Ok(found)
}

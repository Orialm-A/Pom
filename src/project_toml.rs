//! Project TOML module
//!
//! This module handles `pom.toml` file in projects

use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::io_ops::{ExistingFilePolicy, read_file, write_file};
use crate::project_layout::{ModuleLevelSpec, ModuleLevelsMap};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// Intermediary structure to serialize / deserialize the content of `pom.toml` for a project
#[derive(Debug, Serialize, Deserialize)]
pub struct PomToml {
    pub levels: ModuleLevelsMap,
    // Add other project data to save here
}

impl PomToml {
    /// Get the set of root directories of all levels path
    ///
    /// May error `PomErrorCode::PomTomlLevelPathNotNormal`
    pub fn get_modules_roots(&self) -> PomResult<HashSet<PathBuf>> {
        let levels_iterator: Vec<&ModuleLevelSpec> = self.levels.values().collect();
        let mut module_roots_set = HashSet::new();

        for level in levels_iterator {
            let module_root = Self::first_component_of_level_path(&level.path)?;
            module_roots_set.insert(module_root);
        }

        Ok(module_roots_set)
    }

    /// Check if a relative path is normal and return its first component
    ///
    /// May error `PomErrorCode::PomTomlLevelPathNotNormal`
    fn first_component_of_level_path(path: &Path) -> PomResult<PathBuf> {
        let mut components = path.components();

        let first = match components.next() {
            Some(Component::Normal(part)) => PathBuf::from(part),
            _ => {
                return Err((
                    PomErrorCode::PomTomlLevelPathNotNormal,
                    Some(path.display().to_string()),
                ));
            }
        };

        for component in components {
            match component {
                Component::Normal(_) => {}
                _ => {
                    return Err((
                        PomErrorCode::PomTomlLevelPathNotNormal,
                        Some(path.display().to_string()),
                    ));
                }
            }
        }

        Ok(first)
    }
}

/// Check the presence of project `pom.toml` in a project directory and return its content
///
/// May error
pub fn resolve_project_toml(project_root: &Path) -> PomResult<PomToml> {
    if !project_root.is_dir() {
        return Err((
            PomErrorCode::PathToProjectRootExistsAndNotDir,
            Some(project_root.display().to_string()),
        ));
    }

    let pom_toml_path = project_root.join("pom.toml");

    if !pom_toml_path.exists() {
        return Err((
            PomErrorCode::PomTomlNotFound,
            Some(pom_toml_path.display().to_string()),
        ));
    }

    if !pom_toml_path.is_file() {
        return Err((
            PomErrorCode::PomTomlNotFile,
            Some(pom_toml_path.display().to_string()),
        ));
    }

    let pom_toml_str = match read_file(&pom_toml_path) {
        Ok(pom_toml_str) => pom_toml_str,
        Err((_, hint)) => {
            return Err((PomErrorCode::PomTomlFileCantOpen, hint));
        }
    };

    let pom_toml: PomToml = match toml::from_str(&pom_toml_str) {
        Ok(pom_toml) => pom_toml,
        Err(src) => {
            return Err((
                PomErrorCode::PomTomlFileDeserializationFail,
                Some(src.to_string()),
            ));
        }
    };

    Ok(pom_toml)
}

pub fn create_pom_toml_file(
    project_root: &Path,
    module_levels_list: &HashMap<String, ModuleLevelSpec>,
) -> PomResult<()> {
    let pom_toml_file_path = project_root.join("pom.toml");

    let pom_toml = PomToml {
        levels: module_levels_list.clone(),
    };
    let pom_toml_file_content = toml::to_string_pretty(&pom_toml).map_err(|err| {
        (
            PomErrorCode::PomTomlFileSerializationFail,
            Some(err.to_string()),
        )
    })?;

    write_file(
        &pom_toml_file_path,
        &pom_toml_file_content,
        &ExistingFilePolicy::Fail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn minimal_valid_pom_toml() -> String {
        r#"
[levels.app]
path = "src/app"
prefix = "a"

[levels.hld]
path = "src/hld"
prefix = "h"

[levels.lld]
path = "src/lld"
prefix = "l"

[levels.unit_tests]
path = "unit_tests"
prefix = "u"
"#
        .to_string()
    }

    #[test]
    fn resolve_project_toml_ok_minimal() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("pom.toml"), minimal_valid_pom_toml()).unwrap();

        let res = resolve_project_toml(root);
        assert!(res.is_ok(), "Expected Ok(..), got: {:?}", res);
    }

    #[test]
    fn resolve_project_toml_err_not_found() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let err = resolve_project_toml(root).unwrap_err();
        assert_eq!(err.0, PomErrorCode::PomTomlNotFound);
    }

    #[test]
    fn resolve_project_toml_err_pom_toml_not_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("pom.toml")).unwrap();

        let err = resolve_project_toml(root).unwrap_err();
        assert_eq!(err.0, PomErrorCode::PomTomlNotFile);
    }

    #[test]
    fn resolve_project_toml_err_deserialization_fail() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // invalid TOML on purpose
        fs::write(root.join("pom.toml"), "this is not = toml = [").unwrap();

        let err = resolve_project_toml(root).unwrap_err();
        assert_eq!(err.0, PomErrorCode::PomTomlFileDeserializationFail);
    }

    #[test]
    fn resolve_project_toml_err_project_root_not_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("not_a_dir");
        fs::write(&file_path, "x").unwrap();

        let err = resolve_project_toml(&file_path).unwrap_err();
        assert_eq!(err.0, PomErrorCode::PathToProjectRootExistsAndNotDir);
    }

    mod realtive_paths_validation {
        use super::*;

        #[test]
        fn extract_first_component() {
            let test_path = Path::new("this/is/a/path");
            let first_component = PomToml::first_component_of_level_path(test_path).unwrap();
            assert_eq!(first_component, Path::new("this").to_path_buf());

            let test_path = Path::new("this");
            let first_component = PomToml::first_component_of_level_path(test_path).unwrap();
            assert_eq!(first_component, Path::new("this").to_path_buf());
        }

        #[test]
        fn reject_not_normal() {
            let test_path = Path::new("/this/is/a/path/from/root");
            let err = PomToml::first_component_of_level_path(test_path).unwrap_err();
            assert_eq!(err.0, PomErrorCode::PomTomlLevelPathNotNormal);

            let test_path = Path::new("../this/is/a/path/with/parent");
            let err = PomToml::first_component_of_level_path(test_path).unwrap_err();
            assert_eq!(err.0, PomErrorCode::PomTomlLevelPathNotNormal);
        }

        #[test]
        fn extract_all_modules_roots() {
            let dir = tempdir().unwrap();
            let temp_project_dir = dir.path();
            let temp_project_toml_path = temp_project_dir.join("pom.toml");

            fs::write(&temp_project_toml_path, minimal_valid_pom_toml()).unwrap();

            let temp_project_toml = resolve_project_toml(&temp_project_dir).unwrap();

            let modules_roots = PomToml::get_modules_roots(temp_project_toml).unwrap();
            let mut modules_roots_expected = HashSet::new();
            modules_roots_expected.insert(PathBuf::from("src"));
            modules_roots_expected.insert(PathBuf::from("unit_tests"));
            assert_eq!(modules_roots, modules_roots_expected);
        }
    }
}

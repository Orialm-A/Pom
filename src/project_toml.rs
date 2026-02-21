//! Project TOML module
//!
//! This module handles `pom.toml` file in projects

use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::{ExistingFilePolicy, write_file};
use crate::project_layout::{ModuleLevelSpec, ModuleLevelsMap};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Intermediary structure to serialize / deserialize the content of `pom.toml` for a project
#[derive(Debug, Serialize, Deserialize)]
pub struct PomToml {
    pub levels: ModuleLevelsMap,
    // Add other project data to save here
}

/// Check the presence of project `pom.toml` and return its content
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

    let pom_toml_str = match fs::read_to_string(&pom_toml_path) {
        Ok(pom_toml_str) => pom_toml_str,
        Err(src) => {
            return Err((
                PomErrorCode::PomTomlFileCantOpen,
                Some(format!("{}.\r\n{}", pom_toml_path.display(), src)),
            ));
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
}

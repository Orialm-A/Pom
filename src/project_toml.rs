//! Project TOML module
//!
//! This module handles `pom.toml` file in projects

use crate::errors::{PomResult, PomErrorCode};
use std::path::{Path};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::project_layout::{ModuleLevelSpec};
use std::fs;



#[derive(Debug, Serialize, Deserialize)]
pub struct PomToml {
    pub levels: HashMap<String, ModuleLevelSpec>,
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

    let pom_toml_str = match fs::read_to_string(pom_toml_path) {
        Ok(pom_toml_str) => pom_toml_str,
        Err(src) => {
            return Err((
            PomErrorCode::PomTomlFileCantOpen,
            Some(src.to_string()),
        ));
        }
    };

    let pom_toml: PomToml = match toml::from_str(&pom_toml_str) {
        Ok(pom_toml) => pom_toml,
        Err(src) => {
            return Err((
                PomErrorCode::PomTomlFileDeserializationFail,
                Some(src.to_string()),
            ))
        }
    };

    Ok(pom_toml)
}

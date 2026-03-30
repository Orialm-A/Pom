//! Project Layout module
//!
//! This module is responsible for accessing the project layout file and extracting/formating all the
//! pertinent info to create a project

use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::io_ops::read_file;
use convert_case::{Case, Casing};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
/// Describe an entry in the project layout, used to generate file tree and files
struct GenerationLayoutEntry {
    pub path: PathBuf,
    pub defgroup: Option<String>,
    pub brief: Option<String>,
    #[serde(default)]
    pub contains_modules: bool,
    pub module_prefix: Option<String>,
}

#[derive(Debug)]
/// Describe a Doxygen group to generate `doc_groups.h`
pub struct DoxygenGroup {
    pub name: String,
    pub defgroup: Option<String>,
    pub brief: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Describe a module specs for `pom module create`
pub struct ModuleLevelSpec {
    pub path: PathBuf, // Store paths relatively to project root to ensure portability across different computers (`git clone`)
    pub prefix: Option<String>,
}

/// Represents the map of levels name and spec
pub type ModuleLevelsMap = HashMap<String, ModuleLevelSpec>;

/// Represents the data extracted from the project layout
pub struct ResolvedProjectLayout {
    pub dirs: Vec<PathBuf>,
    pub doxygen_groups: Vec<DoxygenGroup>,
    pub module_levels: HashMap<String, ModuleLevelSpec>,
}

#[derive(Debug, Deserialize)]
struct TomlOutput {
    dir: Vec<GenerationLayoutEntry>,
}

/// Resolve project layout for project creation
pub fn resolve_project_layout(project_root: &Path) -> PomResult<ResolvedProjectLayout> {
    let generation_layout = get_generation_layout()?;

    let mut subdirs_list: Vec<PathBuf> = Vec::new();
    // let mut names_list: Vec<String> = Vec::new();
    let mut doxygen_groups_list: Vec<DoxygenGroup> = Vec::new();
    let mut module_levels_map: HashMap<String, ModuleLevelSpec> = HashMap::new();

    for generation_layout_entry in generation_layout {
        let subdir_full_path: PathBuf = project_root.join(&generation_layout_entry.path);

        let dir_name = get_dir_name(&subdir_full_path)?;
        // names_list.push(dir_name.to_string());

        if let Some(group) = get_doxygen_group(&generation_layout_entry, dir_name) {
            doxygen_groups_list.push(group);
        }

        if let Some(module_level_spec) = get_module_level_spec(&generation_layout_entry) {
            module_levels_map.insert(dir_name.to_string(), module_level_spec);
        }

        subdirs_list.push(subdir_full_path);
    }

    Ok(ResolvedProjectLayout {
        dirs: subdirs_list,
        // names: names_list,
        doxygen_groups: doxygen_groups_list,
        module_levels: module_levels_map,
    })
}

fn get_dir_name(path: &Path) -> PomResult<&str> {
    let dir_name_opt = path
        .components()
        .next_back() // Prefer this to `.next()` when using a double ended iterator. `.next()` iterates over all the collection
        .and_then(|c| c.as_os_str().to_str());

    match dir_name_opt {
        Some(dir_name) if !dir_name.is_empty() => Ok(dir_name),
        _ => Err((
            PomErrorCode::GenerationLayoutFileInvalidEntryPath,
            Some(format!("Invalid directory path: `{}`.", path.display())),
        )),
    }
}

fn get_generation_layout() -> PomResult<Vec<GenerationLayoutEntry>> {
    let generation_layout_sources: Vec<PathBuf> = Vec::from([
        // Later: add higher priority config files here
        PathBuf::from("assets/default_generation_layout.toml"), // Lowest priority
    ]);

    let mut generation_layout: Option<TomlOutput> = None;

    for generation_layout_source in generation_layout_sources {
        match get_generation_layout_from_file(&generation_layout_source) {
            Ok(extracted_generation_layout) => {
                // Shadowing doesn't work there
                generation_layout = Some(extracted_generation_layout);
                println!(
                    "Start project generation using {}...",
                    generation_layout_source.display()
                );
                break; // stop at first valid source
            }
            Err((PomErrorCode::GenerationLayoutFileCandidateNotFound, _)) => {
                continue; // try lower priority source
            }
            Err(e) => return Err(e), // Propagate to caller without unpacking
        }
    }

    match generation_layout {
        None => Err((PomErrorCode::GenerationLayoutFileCouldNotFindAny, None)),
        Some(generation_layout) => Ok(generation_layout.dir),
    }
}

fn get_generation_layout_from_file(generation_layout_file_path: &Path) -> PomResult<TomlOutput> {
    let generation_layout_str = match read_file(generation_layout_file_path) {
        Ok(generation_layout_str) => generation_layout_str,
        Err((code, hint)) => {
            return match code {
                PomErrorCode::FilesystemReadTargetNotFound => {
                    Err((PomErrorCode::GenerationLayoutFileCandidateNotFound, None))
                }
                _ => Err((PomErrorCode::GenerationLayoutFileCantOpen, hint)),
            };
        }
    };

    let generation_layout: TomlOutput = toml::from_str(&generation_layout_str).map_err(|e| {
        (
            PomErrorCode::GenerationLayoutFileCantRead,
            Some(e.to_string()),
        )
    })?;

    Ok(generation_layout)
}

fn get_doxygen_group(dir: &GenerationLayoutEntry, dir_name: &str) -> Option<DoxygenGroup> {
    if dir.defgroup.is_none() && dir.brief.is_none() {
        return None;
    }

    Some(DoxygenGroup {
        name: dir_name.to_string().to_case(Case::Snake),
        defgroup: dir.defgroup.clone(),
        brief: dir.brief.clone(),
    })
}

fn get_module_level_spec(
    generation_layout_entry: &GenerationLayoutEntry,
) -> Option<ModuleLevelSpec> {
    if generation_layout_entry.contains_modules {
        Some(ModuleLevelSpec {
            path: generation_layout_entry.path.clone(),
            prefix: generation_layout_entry.module_prefix.clone(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod dir_name_extraction {
        use super::*;

        #[test]
        fn extracts_last_component_normal_path() {
            let p = std::path::Path::new("src/app");
            let name = get_dir_name(p).unwrap();
            assert_eq!(name, "app");
        }

        #[test]
        fn extracts_last_component_with_trailing_slash() {
            let p = std::path::Path::new("src/app/");
            let name = get_dir_name(p).unwrap();
            assert_eq!(name, "app");
        }

        #[test]
        fn fails_on_empty_path() {
            let p = std::path::Path::new("");
            let err = get_dir_name(p).unwrap_err();
            assert_eq!(err.0, PomErrorCode::GenerationLayoutFileInvalidEntryPath);
        }
    }

    mod doxygen_groups_generation {
        use super::*;

        #[test]
        fn returns_none_when_no_defgroup_and_no_brief() {
            let entry = GenerationLayoutEntry {
                path: "src/app".into(),
                defgroup: None,
                brief: None,
                contains_modules: false,
                module_prefix: None,
            };

            let group = get_doxygen_group(&entry, "app");
            assert!(group.is_none());
        }

        #[test]
        fn creates_group_when_defgroup_or_brief_present() {
            let entry = GenerationLayoutEntry {
                path: "src/app".into(),
                defgroup: Some("Application Layer".into()),
                brief: Some("High-level behavior.".into()),
                contains_modules: false,
                module_prefix: None,
            };

            let group = get_doxygen_group(&entry, "app").unwrap();
            assert_eq!(group.name, "app");
            assert_eq!(group.defgroup.as_deref(), Some("Application Layer"));
            assert_eq!(group.brief.as_deref(), Some("High-level behavior."));
        }
    }
}

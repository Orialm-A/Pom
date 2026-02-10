use serde::Deserialize;
use std::fs;
use crate::errors::{PomErrorCode, PomResult};
use std::io::ErrorKind;
use std::path::{PathBuf, Path};
use serde::Serialize;
use std::collections::HashMap;
use convert_case::{Case, Casing};


#[derive(Debug, Deserialize)]
struct GenerationLayoutEntry {
    pub path: String,
    pub defgroup: Option<String>,
    pub brief: Option<String>,
    #[serde(default)]
    pub contains_modules: bool,
    pub module_prefix: Option<String>,
}


#[derive(Debug)]
pub struct DoxygenGroup {
    pub name: String,
    pub defgroup: Option<String>,
    pub brief: Option<String>,
}


#[derive(Debug, Clone, Serialize)]
pub struct ModuleLevelSpec {
    pub path: String, // Store paths relatively to project root to ensure portability across different computers (`git clone`)
    pub prefix: Option<String>,
}


pub struct ResolvedProjectLayout {
    pub dirs: Vec<PathBuf>,
    pub doxygen_groups: Vec<DoxygenGroup>,
    pub module_levels: HashMap<String, ModuleLevelSpec>,
}


#[derive(Debug, Deserialize)]
struct TomlOutput { dir: Vec<GenerationLayoutEntry> }


pub fn resolve_project_layout(project_root: &Path) -> PomResult<ResolvedProjectLayout> {

    let generation_layout = read_generation_layout()?;

    let mut subdirs_list: Vec<PathBuf> = Vec::new();
    // let mut names_list: Vec<String> = Vec::new();
    let mut doxygen_groups_list: Vec<DoxygenGroup> = Vec::new();
    let mut module_levels_map: HashMap<String, ModuleLevelSpec> = HashMap::new();

    for generation_layout_entry in generation_layout {

        let subdir_full_path: PathBuf = project_root.join(&generation_layout_entry.path);


        let dir_name = match get_dir_name(&subdir_full_path) {
            Ok(extracted_dir_name) => extracted_dir_name,
            Err(e) => return Err(e), // Propagate to caller without unpacking
        };
        // names_list.push(dir_name.to_string());

        if let Some(group) = get_doxygen_group(&generation_layout_entry, &dir_name) {
            doxygen_groups_list.push(group);
        }

        if let Some(module_level_spec) = get_module_level_spec(&generation_layout_entry) {
            module_levels_map.insert(dir_name.to_string(), module_level_spec);
        }

        subdirs_list.push(subdir_full_path);
    }

    Ok( ResolvedProjectLayout{
        dirs: subdirs_list,
        // names: names_list,
        doxygen_groups: doxygen_groups_list,
        module_levels: module_levels_map,
    })
}


fn get_dir_name(path: &Path) -> PomResult<&str> {
    let dir_name_opt = path
        .components()
        .last()
        .and_then(|c| c.as_os_str().to_str());

    match dir_name_opt {
        Some(dir_name) if !dir_name.is_empty() => Ok(dir_name),
        _ => Err((
            PomErrorCode::GenerationLayoutFileInvalidEntryPath,
            Some(format!("Invalid directory path: `{}`.", path.display())),
        )),
    }
}


fn read_generation_layout() -> PomResult<Vec<GenerationLayoutEntry>> {
    let dir_tree_sources: Vec<&str> = vec![
        // Later: add higher priority config files here
        "assets/default_generation_layout.toml", // Lowest priority
    ];

    let mut dir_tree: Option<TomlOutput> = None;

    for dir_tree_source in dir_tree_sources {
        match resolve_dir_tree(dir_tree_source) {
            Ok(extracted_dir_tree) => {
                dir_tree = Some(extracted_dir_tree);
                println!("Start project generation using {}...", dir_tree_source);
                break; // stop at first valid source
            }
            Err((PomErrorCode::GenerationLayoutFileCandidateNotFound, _)) => {
                continue; // try lower priority source
            }
            Err(e) => return Err(e), // Propagate to caller without unpacking
        }
    }

    match dir_tree {
        None => Err((
            PomErrorCode::GenerationLayoutFileCouldNotFindAny,
            None,
        )),
        Some(extracted_dir_tree) => Ok(extracted_dir_tree.dir),
    }
}


fn resolve_dir_tree(dir_tree_path_str: &str) -> PomResult<TomlOutput> {

    let dir_tree_str = match fs::read_to_string(dir_tree_path_str) {
        Ok(extracted_string) => extracted_string,
        Err(src) => {
            return match src.kind() {
                ErrorKind::NotFound => Err((PomErrorCode::GenerationLayoutFileCandidateNotFound, None)),
                _ => Err((
                    PomErrorCode::GenerationLayoutFileCantRead,
                    Some(src.to_string())  // `src` type is `Error` which implement `Display`
                )),
            };
        }
    };

    let dir_tree: TomlOutput = match toml::from_str(&dir_tree_str){
        Ok(extracted_dir_tree) => { extracted_dir_tree },
        Err(src) => {
            return Err((
                PomErrorCode::GenerationLayoutFileCantParse,
                Some(src.to_string())  // `src` type is `Error` which implement `Display`
            ));
        },
    };

    Ok(dir_tree)
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


fn get_module_level_spec(generation_layout_entry: &GenerationLayoutEntry) -> Option<ModuleLevelSpec> {
    if generation_layout_entry.contains_modules {
        Some(ModuleLevelSpec {
            path: generation_layout_entry.path.to_string(),
            prefix: generation_layout_entry.module_prefix.clone(),
        })
    } else { None }
}

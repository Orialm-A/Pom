use std::env;
use std::path::{PathBuf, Path};
use crate::errors::{PomErrorCode, PomResult};
use crate::prompt::{prompt_if_missing_string, slugify_snake};
use std::fs;
use crate::read_config_files::{read_generation_layout, GenerationLayoutEntry};
use convert_case::{Case, Casing};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use serde::Serialize;



#[derive(Debug)]
struct DoxygenGroup {
    name: String,
    defgroup: Option<String>,
    brief: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleLevelSpec {
    pub path: String, // Store paths relatively to project root to ensure portability across different computers (`git clone`)
    pub prefix: Option<String>,
}

struct ResolvedProjectLayout {
    dirs: Vec<PathBuf>,
    names: Vec<String>,
    doxygen_groups: Vec<DoxygenGroup>,
    module_levels: HashMap<String, ModuleLevelSpec>,
}

#[derive(Debug, Serialize)]
pub struct PomToml<'a> {
    pub levels: &'a HashMap<String, ModuleLevelSpec>,
    // Add other project data to save here
}



pub fn project_create(
    // The project name passed in the CLI
    project_name_parameter: Option<String>,
    // The path where to create the project passed in the CLI
    project_root_parameter: Option<PathBuf>,
    // Print what would be done without creating / modifying files
    dry_run: bool
) {

    let generation_layout = match read_generation_layout() {
        Ok(extracted_generation_layout) => {extracted_generation_layout},
        Err((error_code, src)) => { error_code.handler(src.as_deref()); }
    };

    let project_root = match get_project_root(project_root_parameter) {
        Ok(extracted_project_root) => { extracted_project_root },
        Err((error_code, src)) => { error_code.handler(src.as_deref()); }
    };

    match validate_project_root(&project_root) {
        Ok(()) => {},
        Err((error_code, src)) => { error_code.handler(src.as_deref()); }
    };

    let (project_name, project_name_normalized) = get_project_name(project_name_parameter);

    let project_root = project_root.join(&project_name_normalized);

    println!(
        "Generate project directory `{}`...",
        project_root.display()
    );

    if !dry_run {
        match create_root_dir(&project_root) {
            Ok(()) => {},
            Err((error_code, src)) => {error_code.handler(src.as_deref()); }
        }
    }

    let resolved_project_layout = match resolve_project_layout(&project_root, &generation_layout) {
        Ok(extracted_resolved_project_layout) => { extracted_resolved_project_layout },
        Err((error_code, src)) => { error_code.handler(src.as_deref()); }
    };

    println!("Generate subdirectories...");
    if !dry_run {
        match create_sub_dirs(&resolved_project_layout.dirs) {
            Ok(()) => {},
            Err((error_code, src)) => {error_code.handler(src.as_deref()); }
        }
    }

    println!("Generate `doc_groups.h`...");
    if !dry_run {
        match generate_doc_groups_file(&project_root, &resolved_project_layout.doxygen_groups) {
            Ok(()) => {},
            Err((error_code, src)) => {error_code.handler(src.as_deref()); }
        }
    }

    println!("Generate `pom.toml`...");
    if !dry_run {
        match generate_pom_toml_file(&project_root, &resolved_project_layout.module_levels) {
            Ok(()) => {},
            Err((error_code, src)) => {error_code.handler(src.as_deref()); }
        }
    }
}


fn get_project_root(project_root_parameter: Option<PathBuf>) -> PomResult<PathBuf> {
    // Path in environment variable is tested first to return early (dev highest priority)
    match env::var("POM_DEV_TEST_PROJECT") {
        Ok(project_root) => return Ok(PathBuf::from(project_root)),
        Err(env::VarError::NotPresent) => {},
        Err(env::VarError::NotUnicode(src)) => {
            let details = format!("{:?}", src);  // `OsString`. Doesn't implement `Display`
            return Err((
                PomErrorCode::PathToProjectRootEnvVarNotUnicode,
                Some(details),
            ));
        },
    };

    // Parameter is tested before local directory to return early ensure user input priority
    match project_root_parameter {
        Some(project_root) => return Ok(project_root),
        None => {},
    };

    // Current directory fallback
    match env::current_dir() {
        Ok(project_root) => Ok(project_root),
        Err(src) => {
            Err((PomErrorCode::PathToProjectRootCantGetCurrentDir, Some(src.to_string())))
        },
    }
}


fn validate_project_root(project_root: &Path) -> PomResult<()> {
    let stringified_project_root = project_root.as_os_str().to_string_lossy();

    if stringified_project_root.trim().is_empty() {
        return Err((PomErrorCode::PathToProjectRootEmpty, None));
    }

    let marker = project_root.join("pom_source_safeguard.txt");
    if marker.is_file() {
        return Err((PomErrorCode::PathToProjectRootInPomSource, None));
    }

    Ok(())
}


fn get_project_name(project_name_parameter: Option<String>) -> (String, String) {
    // `project_name` is to be used in documents read by humans, like README.md
    let project_name = prompt_if_missing_string(project_name_parameter, "Project name");

    // `project_name_normalized` is to be used in paths
    let project_name_normalized = slugify_snake(&project_name);

    (project_name, project_name_normalized)
}


fn create_root_dir(project_root: &Path) -> PomResult<()> {  // `PathBuf` owns memory, `Path` is a borrowed view

    if project_root.exists() {
        let show_path = format!("Check `{}`.", project_root.display().to_string());
        if project_root.is_dir() {
            match project_root.read_dir() {
                Ok(mut entries) => {
                    if entries.next().is_some() {
                        return Err((PomErrorCode::PathToProjectRootNotEmpty, Some(show_path)));
                    }
                }
                Err(src) => {
                    return Err((
                        PomErrorCode::PathToProjectRootFailedToReadDir,
                        Some(src.to_string()),
                    ));
                }
            }
        } else {
            // Exists but not a directory
            return Err((PomErrorCode::PathToProjectRootExistsAndNotDir, Some(show_path)));
        }
    }

    match fs::create_dir_all(project_root) {
        Ok(()) => Ok(()),
        Err(src) => {
            let details = format!("{:?}", src);
            Err((PomErrorCode::PathToProjectRootFailedToCreateRoot, Some(details)))
        }
    }
}


fn resolve_project_layout(project_root: &Path, generation_layout: &[GenerationLayoutEntry]) -> PomResult<ResolvedProjectLayout> {

    let mut subdirs_list: Vec<PathBuf> = Vec::new();
    let mut names_list: Vec<String> = Vec::new();
    let mut doxygen_groups_list: Vec<DoxygenGroup> = Vec::new();
    let mut module_levels_map: HashMap<String, ModuleLevelSpec> = HashMap::new();

    for generation_layout_entry in generation_layout {

        let subdir_full_path: PathBuf = project_root.join(&generation_layout_entry.path);


        let dir_name = match get_dir_name(&subdir_full_path) {
            Ok(extracted_dir_name) => extracted_dir_name,
            Err(e) => return Err(e), // Propagate to caller without unpacking
        };
        names_list.push(dir_name.to_string());

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
        names: names_list,
        doxygen_groups: doxygen_groups_list,
        module_levels: module_levels_map,
    })
}


fn create_sub_dirs(dirs_path_list: &[PathBuf]) -> PomResult<()> {
    for subdir_path in dirs_path_list {
        match fs::create_dir_all(subdir_path) {
            Ok(()) => { continue },
            Err(src) => {
                return Err((
                    PomErrorCode::ProjectGenerationFailedToCreateSubDir,
                    Some(format!("{}: {}", subdir_path.display(), src))
                ));
            }
        };
    }
    Ok(())
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


fn generate_doc_groups_file(project_root: &Path, groups_list: &[DoxygenGroup]) -> PomResult<()> {
    let doc_groups_file_path = project_root.join("doc_groups.h");

    let mut file = match File::create_new(&doc_groups_file_path) {
        Ok(f) => f,
        Err(src) => {
            return Err((
                PomErrorCode::ProjectGenerationFailedToCreateDocGroups,
                Some(src.to_string()),
            ));
        }
    };

    let mut file_content = String::new();

    for group in groups_list {
        file_content.push_str(&generate_group_block(group));
    }

    match file.write_all(file_content.as_bytes()) {
        Ok(()) => {}
        Err(src) => {
            return Err((
                PomErrorCode::ProjectGenerationGroupsFileWriteFailed,
                Some(src.to_string()),
            ));
        }
    }

    Ok(())
}


fn generate_pom_toml_file(project_root: &Path, module_levels_list: &HashMap<String, ModuleLevelSpec>) -> PomResult<()> {

    let pom_toml = PomToml {
        levels: module_levels_list/*.clone()*/,
    };
    let pom_toml_file_string = toml::to_string_pretty(&pom_toml).map_err(
        |err| (
            PomErrorCode::ProjectGenerationFailedToSerializePomToml,
            Some(err.to_string())
        )
    )?;

    let pom_toml_file_path = project_root.join("pom.toml");

    let mut file = match File::create_new(&pom_toml_file_path) {
        Ok(f) => f,
        Err(src) => {
            return Err((
                PomErrorCode::ProjectGenerationFailedToCreatePomToml,
                Some(src.to_string()),
            ));
        }
    };

    match file.write_all(pom_toml_file_string.as_bytes()) {
        Ok(()) => {}
        Err(src) => {
            return Err((
                PomErrorCode::ProjectGenerationPomTomlFileWriteFailed,
                Some(src.to_string()),
            ));
        }
    }

    Ok(())
}


fn generate_group_block(group: &DoxygenGroup) -> String {
    let mut group_block = String::new();
    group_block.push_str("/**\n");
    let id = &group.name;

    let defgroup = match &group.defgroup {
        Some(t) => t.as_str(),
        None => id.as_str(),
    };
    group_block.push_str(&format!(" * @defgroup {} {}\n", id, defgroup));

    if let Some(brief) = &group.brief {
        group_block.push_str(&format!(" * @brief {}\n", brief));
    }

    group_block.push_str(" */\n\n");

    group_block
}


#[cfg(test)]
mod tests{
    use super::*;
    use std::fs::{self, File};
    use std::path::Path;
    use std::io::Read;

    mod root_creation {
        use super::*;

        // Helper to extract just the error code (keeps asserts clean)
        fn code_of<T>(r: PomResult<T>) -> PomErrorCode {
            match r {
                Ok(_) => panic!("expected Err(..), got Ok(..)"),
                Err((code, _details)) => code,
            }
        }

        #[test]
        fn creates_dir_when_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("new_project");

            assert!(!target.exists());
            create_root_dir(target.as_path()).unwrap();
            assert!(target.is_dir());
        }

        #[test]
        fn succeeds_if_dir_exists_and_empty() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("empty_dir");

            fs::create_dir_all(&target).unwrap();
            assert!(target.is_dir());

            create_root_dir(target.as_path()).unwrap();
            assert!(target.is_dir());
        }

        #[test]
        fn fails_if_dir_exists_and_not_empty() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("non_empty_dir");

            fs::create_dir_all(&target).unwrap();
            File::create(target.join("something.txt")).unwrap();

            let err_code = code_of(create_root_dir(target.as_path()));
            assert_eq!(err_code, PomErrorCode::PathToProjectRootNotEmpty);
        }

        #[test]
        fn fails_if_path_exists_and_is_file() {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("not_a_dir");

            File::create(&target).unwrap();
            assert!(target.is_file());

            let err_code = code_of(create_root_dir(target.as_path()));
            assert_eq!(err_code, PomErrorCode::PathToProjectRootExistsAndNotDir);
        }
    }


    mod subdirs_creation {
        use super::*;

        #[test]
        fn creates_dirs_when_missing() {
            let tmp = tempfile::tempdir().unwrap();
            let project_root = tmp.path();

            let dirs = vec![
                project_root.join("src/app"),
                project_root.join("resources/doc"),
            ];

            create_sub_dirs(&dirs).unwrap();

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

            create_sub_dirs(&dirs).unwrap();

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

            let err = create_sub_dirs(&dirs).unwrap_err();
            assert_eq!(err.0, PomErrorCode::ProjectGenerationFailedToCreateSubDir);
        }
    }

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

        #[test]
        fn group_block_contains_expected_tags() {
            let group = DoxygenGroup {
                name: "app".into(),
                defgroup: Some("Application Layer".into()),
                brief: Some("High-level behavior.".into()),
            };

            let block = generate_group_block(&group);

            // Keep assertions loose (format can evolve)
            assert!(block.contains("@defgroup"));
            assert!(block.contains("app"));
            assert!(block.contains("Application Layer"));
            assert!(block.contains("@brief"));
            assert!(block.contains("High-level behavior."));
        }

        #[test]
        fn writes_doc_groups_file() {
            let tmp = tempfile::tempdir().unwrap();
            let project_root = tmp.path();

            let groups = vec![
                DoxygenGroup {
                    name: "app".into(),
                    defgroup: Some("Application Layer".into()),
                    brief: Some("High-level behavior.".into()),
                },
                DoxygenGroup {
                    name: "hld".into(),
                    defgroup: Some("High-Level Drivers".into()),
                    brief: None,
                },
            ];

            generate_doc_groups_file(project_root, &groups).unwrap();

            let mut contents = String::new();
            File::open(project_root.join("doc_groups.h"))
                .unwrap()
                .read_to_string(&mut contents)
                .unwrap();

            assert!(contents.contains("@defgroup"));
            assert!(contents.contains("app"));
            assert!(contents.contains("hld"));
        }
    }
}

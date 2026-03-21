//! "module add" module
//!
//! This Rust module is used to add a C module to a pom project

use crate::cli::resolution::{
    resolve_module_brief, resolve_module_details, resolve_module_level, resolve_new_module_name,
    resolve_project_root,
};
use crate::errors::{PomErrorCode, PomResult};
use crate::filesystem::{ExistingFilePolicy, copy_with_rendering_helper};
use crate::project_toml::resolve_project_toml;
use crate::template_rendering::{FieldKey, TemplateFields};
use chrono::Datelike;
use std::path::{Path, PathBuf};

pub fn module_add(
    module_name: Option<String>,
    level: Option<String>,
    brief: Option<String>,
    details: Option<String>,
    no_prefix: bool,
    dry_run: bool,
) -> PomResult<()> {
    // Check project
    let project_root = resolve_project_root(None)?;
    let project_toml = resolve_project_toml(&project_root)?;

    // Resolve user parameters
    let (module_path, module_prefix, level_group) =
        resolve_module_level(level, &project_toml.levels)?;
    let level_group = format!("@ingroup {}", level_group);
    let module_full_path = project_root.join(module_path);

    let module_prefix = if no_prefix { None } else { module_prefix };

    let (module_name_normalized, module_header_guard) =
        resolve_new_module_name(module_name, &module_prefix)?;

    let brief_raw = resolve_module_brief(brief)?;
    let brief = if brief_raw.is_empty() {
        String::new()
    } else {
        format!("@brief {}", brief_raw)
    };
    let details = resolve_module_details(details)?;

    let current_year = chrono::Local::now().year().to_string();

    let mut rendering_fields = TemplateFields::new();
    rendering_fields.insert(FieldKey::ModuleNameNormalized, &module_name_normalized);
    rendering_fields.insert(FieldKey::ModuleHeaderGuard, module_header_guard);
    rendering_fields.insert(FieldKey::ModuleDoxygenGroup, level_group);
    rendering_fields.insert(FieldKey::ModuleDoxygenBrief, brief);
    rendering_fields.insert(FieldKey::ModuleDoxygenDetails, details);
    rendering_fields.insert(FieldKey::CurrentYear, current_year);

    let module_template_source_path = get_module_source()?;

    // Action

    if !dry_run {
        create_module_files(
            &module_name_normalized,
            &module_template_source_path,
            &module_full_path,
            &rendering_fields,
        )?;
    }

    Ok(())
}

fn get_module_source() -> PomResult<PathBuf> {
    const MODULE_TEMPLATES_DEFAULT_SOURCE: &str = "assets/module_templates";
    let candidates: [&str; 1] = [MODULE_TEMPLATES_DEFAULT_SOURCE];

    for candidate in candidates {
        let root = Path::new(candidate);

        match validate_module_template_dir(root) {
            Ok(()) => return Ok(root.to_path_buf()),

            Err((PomErrorCode::ModuleTemplateSourceNotFound, _)) => continue,

            // Any other validation failure: stop early (bad candidate)
            Err(e) => return Err(e),
        }
    }

    Err((PomErrorCode::ModuleTemplateCouldNotFindAny, None))
}

fn validate_module_template_dir(root: &Path) -> PomResult<()> {
    if !root.exists() {
        return Err((
            PomErrorCode::ModuleTemplateSourceNotFound,
            Some(root.display().to_string()),
        ));
    }
    if !root.is_dir() {
        return Err((
            PomErrorCode::ModuleTemplateSourceNotDir,
            Some(root.display().to_string()),
        ));
    }

    let c_path = root.join("template.c.pomrt");
    let h_path = root.join("template.h.pomrt");

    if !c_path.is_file() {
        return Err((
            PomErrorCode::ModuleTemplateMissing,
            Some(c_path.display().to_string()),
        ));
    }
    if !h_path.is_file() {
        return Err((
            PomErrorCode::ModuleTemplateMissing,
            Some(h_path.display().to_string()),
        ));
    }

    Ok(())
}

fn create_module_files(
    module_name: &str,
    source_path: &Path,
    module_path: &Path,
    fields: &TemplateFields,
) -> PomResult<()> {
    if !module_path.exists() {
        return Err((
            PomErrorCode::ModuleDestinationDirNotFound,
            Some(module_path.display().to_string()),
        ));
    }
    if !module_path.is_dir() {
        return Err((
            PomErrorCode::ModuleDestinationDirNotDir,
            Some(module_path.display().to_string()),
        ));
    }

    for extension in ["h", "c"] {
        let file_source_path = source_path.join(format!("template.{}.pomrt", extension));
        let file_destination_path = module_path.join(format!("{}.{}", module_name, extension));

        copy_with_rendering_helper(
            &file_source_path,
            &file_destination_path,
            fields,
            &ExistingFilePolicy::Fail,
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn validate_module_template_dir_err_not_found() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope");
        let err = validate_module_template_dir(&missing).unwrap_err();
        assert_eq!(err.0, PomErrorCode::ModuleTemplateSourceNotFound);
    }

    #[test]
    fn validate_module_template_dir_err_not_dir() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("file");
        fs::write(&file, "x").unwrap();

        let err = validate_module_template_dir(&file).unwrap_err();
        assert_eq!(err.0, PomErrorCode::ModuleTemplateSourceNotDir);
    }

    #[test]
    fn validate_module_template_dir_err_missing_templates() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // directory exists but missing required template files
        let err = validate_module_template_dir(root).unwrap_err();
        assert_eq!(err.0, PomErrorCode::ModuleTemplateMissing);
    }

    #[test]
    fn validate_module_template_dir_ok() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::write(root.join("template.c.pomrt"), "C").unwrap();
        fs::write(root.join("template.h.pomrt"), "H").unwrap();

        let res = validate_module_template_dir(root);
        assert!(res.is_ok(), "expected Ok(()), got {:?}", res);
    }

    #[test]
    fn create_module_files_creates_c_and_h() {
        let templates = tempdir().unwrap();
        let out = tempdir().unwrap();

        fs::write(templates.path().join("template.c.pomrt"), "C FILE").unwrap();
        fs::write(templates.path().join("template.h.pomrt"), "H FILE").unwrap();

        let fields = TemplateFields::new(); // no placeholders needed for this test

        create_module_files("my_module", templates.path(), out.path(), &fields).unwrap();

        let c = out.path().join("my_module.c");
        let h = out.path().join("my_module.h");

        assert!(c.is_file());
        assert!(h.is_file());

        assert_eq!(fs::read_to_string(c).unwrap(), "C FILE");
        assert_eq!(fs::read_to_string(h).unwrap(), "H FILE");
    }
}

//! "module add" module
//!
//! This Rust module is used to add a C module to a pom project

use crate::errors::{PomResult, PomErrorCode};
use crate::cli::resolution::{resolve_project_root, resolve_new_module_name, resolve_module_brief, resolve_module_details, resolve_module_level};
use crate::project_toml::{resolve_project_toml};
use crate::template_rendering::{TemplateFields, FieldKey};
use chrono::Datelike;
use std::path::{Path, PathBuf};
use crate::filesystem::{ExistingFilePolicy, copy_with_rendering_helper};


pub fn module_add(
    module_name: Option<String>,
    level: Option<String>,
    brief: Option<String>,
    details: Option<String>,
    dry_run: bool,
) -> PomResult<()> {

    // Check project
    let project_root = resolve_project_root(None)?;
    let project_toml = resolve_project_toml(&project_root)?;

    // Resolve user parameters
    let (module_path, module_prefix, level_group) = resolve_module_level(level, &project_toml.levels)?;
    let level_group = format!("@ingroup {}", level_group);
    let module_full_path = project_root.join(module_path);

    let (module_name_normalized, module_header_guard) = resolve_new_module_name(module_name, &module_prefix)?;

    let brief = format!("@brief {}", resolve_module_brief(brief)?);
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
        create_module_files(&module_name_normalized, &module_template_source_path, &module_full_path, &rendering_fields)?;
    }

    Ok(())
}


fn get_module_source() -> PomResult<PathBuf> {
    const MODULE_TEMPLATES_DEFAULT_SOURCE: &'static str = "assets/module_templates";
    let candidates: [&str; 1] = [
        MODULE_TEMPLATES_DEFAULT_SOURCE,
    ];

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

fn create_module_files(module_name: &str, source_path: &Path, module_path: &Path, fields: &TemplateFields) -> PomResult<()> {
    for extension in ["h", "c"] {
        let file_source_path = source_path.join(format!("template.{}.pomrt", extension));
        let file_destination_path = module_path.join(format!("{}.{}", module_name, extension));
        copy_with_rendering_helper(&file_source_path, &file_destination_path, &fields, &ExistingFilePolicy::Fail)?;
    }
    Ok(())
}

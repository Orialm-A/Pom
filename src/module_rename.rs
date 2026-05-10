//! `module rename` command implementation.
//!
//! Provides the CLI entry point and supporting functions to rename a C
//! module in a Pom project.

use crate::cli::resolution::{
    resolve_new_module_name, resolve_old_module_name, resolve_project_root,
};
use crate::errors::PomResult;
use crate::filesystem::browsing::{EntryKind, validate_dir_entry};
use crate::filesystem::io_ops::{ExistingFilePolicy, read_file, rename_file, write_file};
use crate::project_toml::resolve_project_toml;
use crate::prompt::confirm;
use owo_colors::OwoColorize;
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// Use this struct to pass all the parameters to the rename flow, after the interactive functions, to make it testable
struct ModuleRenameContext<'a> {
    project_root: &'a Path,
    module_path: &'a Path,
    old_name: &'a str,
    new_name: &'a str,
    old_name_was_unique: bool,
    old_header_guard: &'a str,
    new_header_guard: &'a str,
    header_file: bool,
    source_file: bool,
    search_scope: &'a HashSet<PathBuf>,
}

/// Result of a module rename operation
///
/// This enum indicates whether the rename process has been applied or aborted
/// by the user.
/// It is used to determine whether follow-up actions, such as include updates,
/// should be executed.
#[derive(PartialEq)]
enum RenameOutcome {
    Applied,
    Aborted,
}

/// CLI `module rename` entry point
pub fn module_rename(
    old_module_name: Option<String>,
    new_module_name: Option<String>,
    skip_confirmation: bool,
) -> PomResult<()> {
    let project_root = resolve_project_root(None)?;
    let project_toml = resolve_project_toml(&project_root)?;
    let search_scope = project_toml.get_modules_roots()?;

    let (module_path, old_module_name_normalized, old_header_guard, module_files, module_is_unique) =
        resolve_old_module_name(old_module_name, &project_root, &search_scope)?;
    let (new_module_name_normalized, new_header_guard) =
        resolve_new_module_name(new_module_name, &None, &project_root, &search_scope)?;
    let skip_confirmation = if skip_confirmation { Some(true) } else { None };

    let rename_context = ModuleRenameContext {
        project_root: &project_root,
        module_path: &module_path,
        old_name: &old_module_name_normalized,
        new_name: &new_module_name_normalized,
        old_name_was_unique: module_is_unique,
        old_header_guard: &old_header_guard,
        new_header_guard: &new_header_guard,
        header_file: module_files.header_file,
        source_file: module_files.source_file,
        search_scope: &search_scope,
    };

    let rename_outcome = module_rename_flow(&rename_context, skip_confirmation)?;

    if rename_outcome == RenameOutcome::Applied {
        includes_update_flow(&rename_context)?;
    }

    Ok(())
}

/// Rename logic, isolated to make it testable without prompts
fn module_rename_flow(
    ctx: &ModuleRenameContext,
    auto_confirm: Option<bool>,
) -> PomResult<RenameOutcome> {
    println!(
        "Trying to rename module `{}` into `{}` in `{}`...",
        ctx.old_name,
        ctx.new_name,
        ctx.module_path.display()
    );

    let module_path = ctx.project_root.join(ctx.module_path);

    // Defining these paths doesn't hurt, as the functions using them ("write", "read", "rename"...) check for existence
    // Making them `Option<PathBuf>` and conditional on `module_files` would hurt readability, with dereferencing everywhere
    let old_header_path = module_path.join(format!("{}.h", ctx.old_name));
    let old_source_path = module_path.join(format!("{}.c", ctx.old_name));
    let new_header_path = module_path.join(format!("{}.h", ctx.new_name));
    let new_source_path = module_path.join(format!("{}.c", ctx.new_name));

    let header_content = if ctx.header_file {
        inspect_header(&old_header_path, ctx.old_header_guard, ctx.new_header_guard)?
    } else {
        println!("Note: Header file not found for this module.");
        None
    };

    let source_content = if ctx.source_file {
        inspect_source(&old_source_path, ctx.old_name, ctx.new_name)?
    } else {
        println!("Note: Source file not found for this module.");
        None
    };

    // User get prompted after all the warnings got a chance to be displayed
    let rename_confirmation = match auto_confirm {
        Some(auto_confirm) => auto_confirm,
        None => confirm("Proceed with renaming?")?,
    };

    if rename_confirmation {
        // Do not make a helper for this block, it would take a 8-parameters function, which doesn't simplify reading
        if ctx.header_file {
            apply_rename(
                &old_header_path,
                &new_header_path,
                header_content.as_deref(),
            )?;
        }

        if ctx.source_file {
            apply_rename(
                &old_source_path,
                &new_source_path,
                source_content.as_deref(),
            )?;
        }

        Ok(RenameOutcome::Applied)
    } else {
        println!("Rename aborted - No file had been modified.");
        Ok(RenameOutcome::Aborted)
    }
}

/// Inspect a header file and prepare a header guard update.
///
/// If the header guard is found, returns the updated file content with all
/// occurrences of `old_header_guard` replaced by `new_header_guard`.
/// Else, returns `Ok(None)` and emits a warning.
///
/// This function does not modify the file on disk.
///
/// # Errors
///
/// Returns any error that occurs while reading the file.
fn inspect_header(
    header_path: &Path,
    old_header_guard: &str,
    new_header_guard: &str,
) -> PomResult<Option<String>> {
    let result = inspect_file(header_path, old_header_guard, new_header_guard)?;

    if result.is_none() {
        println!("Warning: Header Guard not found in header file.");
    }

    Ok(result)
}

/// Inspect a source file and prepare a self-header include update.
///
/// If the self-header include is found, returns the updated file content with all
/// occurrences of `old_module_name` replaced by `new_module_name`.
/// Else, returns `Ok(None)` and emits a warning.
///
/// This function does not modify the file on disk.
///
/// # Errors
///
/// Returns any error that occurs while reading the file.
fn inspect_source(
    source_path: &Path,
    old_module_name: &str,
    new_module_name: &str,
) -> PomResult<Option<String>> {
    let old_header_file_name = format!("{}.h", old_module_name);
    let new_header_file_name = format!("{}.h", new_module_name);
    let result = inspect_file(source_path, &old_header_file_name, &new_header_file_name)?;

    if result.is_none() {
        println!("Warning: Self-include not found in source file.");
    }

    Ok(result)
}

/// Inspect a file and prepare a content update by replacing a pattern.
///
/// Reads the file at `file_path` and replaces all occurrences of
/// `string_to_replace` with `replacement_string`.
///
/// Returns:
/// - `Ok(Some(String))` if at least one replacement occurred, containing the
///   updated file content
/// - `Ok(None)` if no occurrence was found (no change needed)
///
/// This function does not modify the file on disk.
///
/// # Errors
///
/// Returns any error that occurs while reading the file.
fn inspect_file(
    file_path: &Path,
    string_to_replace: &str,
    replacement_string: &str,
) -> PomResult<Option<String>> {
    let file_content = read_file(file_path)?;
    let updated_file_content = file_content.replace(string_to_replace, replacement_string);

    if updated_file_content == file_content {
        return Ok(None);
    }

    Ok(Some(updated_file_content))
}

/// Apply the planned changes to the real files
fn apply_rename(old_path: &Path, new_path: &Path, new_content: Option<&str>) -> PomResult<()> {
    if let Some(new_content) = new_content {
        write_file(old_path, new_content, &ExistingFilePolicy::Overwrite)?;
    }
    rename_file(old_path, new_path)?;

    Ok(())
}

/// header includes update flow
///
/// Iterate over the files after a rename. Within a file, iterate over lines to find includes. These includes are processed by the function `process_line()`.
///
/// # Arguments
/// - `ctx` - Struct containing all the parameters for the rename
///
/// # Errors
/// Propagates error from `read_file()` and `process_line()`
fn includes_update_flow(ctx: &ModuleRenameContext) -> PomResult<()> {
    for dir in ctx.search_scope {
        let search_path = ctx.project_root.join(dir);
        for entry in WalkDir::new(search_path).into_iter() {
            let validated_entry = validate_dir_entry(entry, ctx.project_root)?;

            let entry_relative_path = validated_entry.relative_path;
            let entry_full_path = validated_entry.full_path;
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
                    // Read
                    let file_content = read_file(&entry_full_path)?;
                    let new_line_character = if file_content.contains("\r\n") {
                        "\r\n"
                    } else {
                        "\n"
                    };

                    let mut new_lines: Vec<Cow<'_, str>> = Vec::new();
                    let mut updates_count: u32 = 0;

                    for (line_index, line) in file_content.lines().enumerate() {
                        let (new_line, line_modified) =
                            process_include_line(line, &line_index, &entry_relative_path, ctx)?;
                        new_lines.push(new_line);
                        if line_modified {
                            updates_count += 1;
                        }
                    }

                    if updates_count > 0 {
                        let new_content = new_lines.join(new_line_character);
                        write_file(
                            &entry_full_path,
                            &new_content,
                            &ExistingFilePolicy::Overwrite,
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Process a single line and update a `#include` directive if it targets the
/// renamed module.
///
/// This function inspects the given `line` and determines whether it is a
/// relevant `#include` directive referencing the module being renamed. If so,
/// it applies the appropriate update according to Pom’s disambiguation rules.
///
/// The update is performed only when the include can be resolved unambiguously
/// (e.g. project-qualified path or file-relative resolution), or after explicit
/// user confirmation.
///
/// Non-`#include` lines are returned unchanged.
/// `#include` lines not referencing the old module are returned unchanged.
/// Matching includes are updated only if they can be safely resolved or confirmed by the user.
///
/// # Arguments
/// - `line` - The line to process
/// - `line_number` - Index of the line in the file
/// - `current_file_path` - Path to the file containing `line` (without the file name)
/// - `ctx` - Struct containing all the parameters for the rename
///
/// # Returns
/// Returns a tuple:
/// - `Cow<str>` - The updated line if modified, or the original line otherwise
/// - `bool` - `true` if the line was modified, `false` otherwise
///
/// # Errors
/// Propagates any error that occurs during include resolution or user interaction.
fn process_include_line<'a>(
    line: &'a str,
    line_number: &usize,
    current_file_path: &Path,
    ctx: &ModuleRenameContext,
) -> PomResult<(Cow<'a, str>, bool)> {
    let old_header_name = format!("{}.h", ctx.old_name);

    let extracted_include_path = match extract_matching_include_path(line, &old_header_name) {
        Some(include_path) => PathBuf::from(include_path),
        None => {
            return Ok((Cow::Borrowed(line), false));
        }
    };

    let new_header_name = format!("{}.h", ctx.new_name);
    let original_line = Cow::Borrowed(line);
    let modified_line = Cow::Owned(line.replacen(&old_header_name, &new_header_name, 1));
    let old_header_file_path = ctx.module_path.join(&old_header_name);

    let should_update = ctx.old_name_was_unique
        || is_include_fully_resolved(&extracted_include_path, ctx.search_scope)
        || is_include_resolved_from_file(
            &old_header_file_path,
            current_file_path,
            &extracted_include_path,
        )
        || does_user_approve_include_update(
            current_file_path,
            *line_number,
            &original_line,
            &modified_line,
        )?;

    if should_update {
        Ok((modified_line, true))
    } else {
        Ok((original_line, false))
    }
}

/// Extract the include path from a `#include` line if it targets the given header.
///
/// This function parses a line of C source code and attempts to extract the path inside a `#include` directive (either `"..."` or `<...>`). It returns a slice of the original line corresponding to the include path.
///
/// The function only returns a value if:
/// - the line starts with `#include` (after trimming leading whitespace)
/// - the line contains `old_header_name`
/// - the include path is correctly delimited with `"` or `< >`
///
/// # Arguments
/// - `line` - The full line of source code to inspect
/// - `old_header_name` - Name of the header file being searched (e.g. `"timer.h"`)
///
/// # Returns
/// - `Some(&str)` containing the extracted include path (borrowed from `line`)
/// - `None` if the line is not a matching `#include` or cannot be parsed
///
/// # Notes
/// - The returned `&str` is a slice of the input `line` and does not allocate
/// - This function does not validate whether the extracted path exists on disk
/// - Matching is based on a simple substring check and does not handle macros or complex preprocessor constructs
fn extract_matching_include_path<'a>(line: &'a str, old_header_name: &str) -> Option<&'a str> {
    let trimmed = line.trim_start();

    if !trimmed.starts_with("#include") {
        return None;
    }

    if !trimmed.contains(old_header_name) {
        return None;
    }

    let rest = trimmed.trim_start_matches("#include").trim();

    if let Some(include_path) = rest.strip_prefix('"') {
        include_path.find('"').map(|end| &rest[1..1 + end])
    } else if let Some(include_path) = rest.strip_prefix('<') {
        include_path.find('>').map(|end| &rest[1..1 + end])
    } else {
        None
    }
}

/// Determine whether an include path is project-qualified (fully resolved)
/// according to Pom's search scope.
///
/// An include path is considered "fully resolved" if its first path component
/// matches one of the directories in `search_scope`. This means the include
/// explicitly starts from a known project root (e.g. `src/...` or `unit_tests/...`)
/// rather than being relative to the including file.
///
/// Only the first path component is considered.
/// Relative paths such as `./...` or `../...` are rejected.
/// Absolute paths are rejected.
/// Empty paths are rejected.
///
/// # Arguments
/// - `extracted_include_path` - Path extracted from a `#include` directive
/// - `search_scope` - Set of project root directories (e.g. `src`, `unit_tests`)
///
/// # Returns
/// - `true` if the include path starts with a directory present in `search_scope`
/// - `false` otherwise
///
/// # Notes
/// - This function does not access the filesystem; it performs a purely
///   syntactic check
/// - This is a Pom-specific definition of "fully resolved" and does not
///   reflect the full behavior of a C compiler's include resolution
fn is_include_fully_resolved(
    extracted_include_path: &Path,
    search_scope: &HashSet<PathBuf>,
) -> bool {
    match extracted_include_path.components().next() {
        Some(Component::Normal(part)) => search_scope.contains(Path::new(part)),
        _ => false,
    }
}

/// Determine whether an include path resolves to the renamed header when interpreted relative to the including file's directory.
///
/// This function simulates the first step of the compiler's include resolution:
/// resolving a quoted include relative to the directory of the file that contains it.
///
/// It constructs a candidate path by joining the directory of the current file with the extracted include path, and compares it
/// to the known path of the renamed header.
///
/// # Arguments
/// - `old_header_file_path` - Full path to the original header file before rename
/// - `current_file_path` - Full path to the file containing the include
/// - `extracted_include_path` - Include path extracted from the `#include` line
///
/// # Returns
/// - `true` if the include resolves exactly to the renamed header
/// - `false` otherwise
///
/// # Notes
/// - This comparison is purely path-based and does not access the filesystem
/// - The file name is removed from `current_file_path` to obtain its directory
/// - Paths are compared as-is; no normalization (e.g. `.` / `..`) is performed
///
/// # Debug
/// This function currently prints the compared paths for debugging purposes.
/// This should be removed or gated behind a debug flag in production.
fn is_include_resolved_from_file(
    old_header_file_path: &Path,
    current_file_path: &Path,
    extracted_include_path: &Path,
) -> bool {
    let mut current_file_path_components = current_file_path.components();
    _ = current_file_path_components.next_back();
    let current_file_dir_path = current_file_path_components.as_path();
    let resolved_header_file_path = current_file_dir_path.join(extracted_include_path);

    println!(
        "The function `is_include_resolved_from_file()`is comparing these two paths:\r\n - `{}`\r\n - `{}` ( = {} + {})",
        old_header_file_path.display(),
        &resolved_header_file_path.display(),
        current_file_dir_path.display(),
        extracted_include_path.display()
    );

    old_header_file_path == resolved_header_file_path
}

/// Ask the user whether an include line should be updated.
///
/// The prompt displays the file path, line number, original line, and proposed
/// replacement using a diff-like `-` / `+` format.
///
/// # Returns
/// - `true` if the user confirms the update
/// - `false` otherwise
///
/// # Arguments
/// - `current_file_path` - Path to the file containing the include line
/// - `line_number` - Line number of the include line
/// - `old_line` - Current line content
/// - `new_line` - Proposed line content
///
/// # Errors
/// Propagates any error emitted while displaying the confirmation prompt.
fn does_user_approve_include_update(
    current_file_path: &Path,
    line_number: usize,
    old_line: &str,
    new_line: &str,
) -> PomResult<bool> {
    let old_diff_line = format!("- `{}`", old_line);
    let new_diff_line = format!("+ `{}`", new_line);

    let prompt = format!(
        "In {}, update line {}:\r\n{}\r\n{}\r\n",
        current_file_path.display(),
        line_number,
        &old_diff_line.bright_red(),
        &new_diff_line.bright_green(),
    );

    confirm(&prompt)
}

#[cfg(test)]
mod module_rename_tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::{TempDir, tempdir};

    fn create_module_tree() -> (TempDir, PathBuf, PathBuf, HashSet<PathBuf>) {
        let temp_dir = tempdir().unwrap();
        let project_root = temp_dir.path().join("project");
        let module_dir = project_root.join("src/services");
        let mut search_scope: HashSet<PathBuf> = HashSet::new();

        search_scope.insert(PathBuf::from("src"));

        fs::create_dir_all(&module_dir).unwrap();

        let header_path = module_dir.join("module_to_rename.h");
        let source_path = module_dir.join("module_to_rename.c");

        write_file(
            &header_path,
            "#ifndef MODULE_TO_RENAME_H\n#define MODULE_TO_RENAME_H\n\n#endif // MODULE_TO_RENAME_H\n",
            &ExistingFilePolicy::Overwrite,
        )
        .unwrap();

        write_file(
            &source_path,
            "#include \"module_to_rename.h\"\n\nvoid test(void) {}\n",
            &ExistingFilePolicy::Overwrite,
        )
        .unwrap();

        (
            temp_dir,
            project_root,
            PathBuf::from("src/services"),
            search_scope,
        )
    }

    mod file_inspection_tests {
        use super::*;

        #[test]
        fn inspect_file_replaces_all_occurrences() {
            let temp_dir = tempdir().unwrap();
            let file_path = temp_dir.path().join("test.h");

            write_file(
                &file_path,
                "#ifndef OLD_H\n#define OLD_H\n#endif // OLD_H\n",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();

            let updated = inspect_file(&file_path, "OLD_H", "NEW_H").unwrap();

            assert_eq!(
                updated,
                Some("#ifndef NEW_H\n#define NEW_H\n#endif // NEW_H\n".to_string())
            );
        }

        #[test]
        fn inspect_file_returns_none_when_pattern_not_found() {
            let temp_dir = tempdir().unwrap();
            let file_path = temp_dir.path().join("test.c");

            write_file(
                &file_path,
                "#include \"other.h\"\n",
                &ExistingFilePolicy::Overwrite,
            )
            .unwrap();

            let updated = inspect_file(&file_path, "module_to_rename.h", "new_name.h").unwrap();

            assert_eq!(updated, None);
        }
    }

    mod rename_application_tests {
        use super::*;

        #[test]
        fn apply_rename_renames_without_rewriting_when_content_is_none() {
            let temp_dir = tempdir().unwrap();
            let old_path = temp_dir.path().join("old.txt");
            let new_path = temp_dir.path().join("new.txt");

            write_file(&old_path, "Hello", &ExistingFilePolicy::Overwrite).unwrap();

            apply_rename(&old_path, &new_path, None).unwrap();

            assert!(!old_path.exists());
            assert!(new_path.exists());
            assert_eq!(read_file(&new_path).unwrap(), "Hello");
        }

        #[test]
        fn apply_rename_rewrites_then_renames_when_content_is_some() {
            let temp_dir = tempdir().unwrap();
            let old_path = temp_dir.path().join("old.txt");
            let new_path = temp_dir.path().join("new.txt");

            write_file(&old_path, "Hello", &ExistingFilePolicy::Overwrite).unwrap();

            apply_rename(&old_path, &new_path, Some("Updated")).unwrap();

            assert!(!old_path.exists());
            assert!(new_path.exists());
            assert_eq!(read_file(&new_path).unwrap(), "Updated");
        }
    }

    mod rename_confirmation_tests {
        use super::*;

        #[test]
        fn abort_keeps_files_and_content_unchanged() {
            let (_temp_dir, project_root, module_path, search_scope) = create_module_tree();

            let rename_context = ModuleRenameContext {
                project_root: &project_root,
                module_path: &module_path,
                old_name: "module_to_rename",
                new_name: "new_name",
                old_name_was_unique: true,
                old_header_guard: "MODULE_TO_RENAME_H",
                new_header_guard: "NEW_NAME_H",
                header_file: true,
                source_file: true,
                search_scope: &search_scope,
            };

            module_rename_flow(&rename_context, Some(false)).unwrap();

            let old_header = project_root.join("src/services/module_to_rename.h");
            let old_source = project_root.join("src/services/module_to_rename.c");
            let new_header = project_root.join("src/services/new_name.h");
            let new_source = project_root.join("src/services/new_name.c");

            assert!(old_header.exists());
            assert!(old_source.exists());
            assert!(!new_header.exists());
            assert!(!new_source.exists());

            assert_eq!(
                read_file(&old_header).unwrap(),
                "#ifndef MODULE_TO_RENAME_H\n#define MODULE_TO_RENAME_H\n\n#endif // MODULE_TO_RENAME_H\n"
            );
            assert_eq!(
                read_file(&old_source).unwrap(),
                "#include \"module_to_rename.h\"\n\nvoid test(void) {}\n"
            );
        }

        #[test]
        fn confirm_renames_files_and_updates_content() {
            let (_temp_dir, project_root, module_path, search_scope) = create_module_tree();

            let rename_context = ModuleRenameContext {
                project_root: &project_root,
                module_path: &module_path,
                old_name: "module_to_rename",
                new_name: "new_name",
                old_name_was_unique: true,
                old_header_guard: "MODULE_TO_RENAME_H",
                new_header_guard: "NEW_NAME_H",
                header_file: true,
                source_file: true,
                search_scope: &search_scope,
            };

            module_rename_flow(&rename_context, Some(true)).unwrap();

            let old_header = project_root.join("src/services/module_to_rename.h");
            let old_source = project_root.join("src/services/module_to_rename.c");
            let new_header = project_root.join("src/services/new_name.h");
            let new_source = project_root.join("src/services/new_name.c");

            assert!(!old_header.exists());
            assert!(!old_source.exists());
            assert!(new_header.exists());
            assert!(new_source.exists());

            assert_eq!(
                read_file(&new_header).unwrap(),
                "#ifndef NEW_NAME_H\n#define NEW_NAME_H\n\n#endif // NEW_NAME_H\n"
            );
            assert_eq!(
                read_file(&new_source).unwrap(),
                "#include \"new_name.h\"\n\nvoid test(void) {}\n"
            );
        }
    }

    mod include_lines_parsing_tests {
        use super::*;

        #[test]
        fn ignores_non_include_lines() {
            let line = "void test(void) {}";
            let old_module_name = "some_name";
            let result = extract_matching_include_path(&line, &old_module_name);
            assert_eq!(result, None);
        }

        #[test]
        fn ignores_includes_not_containing_module_old_name() {
            let line = "#include \"path/to/a/module.h\"";
            let old_module_name = "some_name";
            let result = extract_matching_include_path(&line, &old_module_name);
            assert_eq!(result, None);
        }

        #[test]
        fn returns_extracted_include_path() {
            let line = "#include \"path/to/some_name.h\"";
            let old_header_name = "some_name.h";
            let result = extract_matching_include_path(line, old_header_name);
            assert_eq!(result, Some("path/to/some_name.h"));

            let line = "#include <path/to/some_name.h>";
            let old_header_name = "some_name.h";
            let result = extract_matching_include_path(line, old_header_name);
            assert_eq!(result, Some("path/to/some_name.h"));
        }
    }

    mod fully_resolved_include_path_tests {
        use super::*;

        #[test]
        fn accepts_include_path_starting_in_search_scope() {
            let mut search_scope: HashSet<PathBuf> = HashSet::new();
            search_scope.insert(PathBuf::from("src"));
            search_scope.insert(PathBuf::from("unit_tests"));

            let extracted_include_path =
                PathBuf::from("src/path/fully/resolved/from/project/sources/to/header.h");
            assert!(is_include_fully_resolved(
                &extracted_include_path,
                &search_scope
            ));
        }

        #[test]
        fn rejects_include_path_not_starting_in_search_scope() {
            let mut search_scope: HashSet<PathBuf> = HashSet::new();
            search_scope.insert(PathBuf::from("src"));
            search_scope.insert(PathBuf::from("unit_tests"));

            let extracted_include_path = PathBuf::from("path/not/fully/resolved/to/header.h");
            assert_eq!(
                is_include_fully_resolved(&extracted_include_path, &search_scope),
                false
            );
        }

        #[test]
        fn rejects_relative_include_paths() {
            let mut search_scope: HashSet<PathBuf> = HashSet::new();
            search_scope.insert(PathBuf::from("src"));
            search_scope.insert(PathBuf::from("unit_tests"));

            let extracted_include_path = PathBuf::from("./path/not/fully/resolved/to/header.h");
            assert_eq!(
                is_include_fully_resolved(&extracted_include_path, &search_scope),
                false
            );

            let extracted_include_path = PathBuf::from("../path/not/fully/resolved/to/header.h");
            assert_eq!(
                is_include_fully_resolved(&extracted_include_path, &search_scope),
                false
            );
        }

        #[test]
        fn rejects_empty_paths() {
            let mut search_scope: HashSet<PathBuf> = HashSet::new();
            search_scope.insert(PathBuf::from("src"));
            search_scope.insert(PathBuf::from("unit_tests"));

            let extracted_include_path = PathBuf::from("");
            assert_eq!(
                is_include_fully_resolved(&extracted_include_path, &search_scope),
                false
            );
        }

        #[test]
        fn rejects_absolute_path() {
            let mut search_scope: HashSet<PathBuf> = HashSet::new();
            search_scope.insert(PathBuf::from("src"));
            search_scope.insert(PathBuf::from("unit_tests"));

            let extracted_include_path = PathBuf::from("/src/header.h");
            assert_eq!(
                is_include_fully_resolved(&extracted_include_path, &search_scope),
                false
            );
        }
    }

    mod include_resolution_from_file_dir_tests {
        use super::*;

        #[test]
        fn resolves_include_relative_to_current_file_dir() {
            let old_header_file_path = PathBuf::from("path/to/a_module/with/old_name.h");
            let current_file = PathBuf::from("path/to/source.c");
            let extracted_include_path = PathBuf::from("a_module/with/old_name.h");

            let result = is_include_resolved_from_file(
                &old_header_file_path,
                &current_file,
                &extracted_include_path,
            );
            assert_eq!(result, true);
        }

        #[test]
        fn rejects_header_with_same_name_in_another_location() {
            let old_header_file_path = PathBuf::from("path/to/a_module/with/old_name.h");
            let current_file = PathBuf::from("path/to/source.c");
            let extracted_include_path = PathBuf::from("another_module/with/old_name.h");

            let result = is_include_resolved_from_file(
                &old_header_file_path,
                &current_file,
                &extracted_include_path,
            );
            assert_eq!(result, false);
        }

        #[test]
        fn rejects_project_relative_include_when_resolving_from_current_file_dir() {
            let old_header_file_path = PathBuf::from("path/to/a_module/with/old_name.h");
            let current_file = PathBuf::from("path/to/source.c");
            let extracted_include_path = PathBuf::from("to/a_module/with/old_name.h");

            let result = is_include_resolved_from_file(
                &old_header_file_path,
                &current_file,
                &extracted_include_path,
            );
            assert_eq!(result, false);
        }
    }
}

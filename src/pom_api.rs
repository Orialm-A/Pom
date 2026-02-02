use crate::project_create::project_create;
use std::path::PathBuf;


pub fn project_create_function(project_name: Option<String>, project_path: Option<PathBuf>, dry_run: bool) {
    // eprintln!("`pom project create` - not implemented yet");
    project_create(project_name, project_path, dry_run);
}

pub fn project_rename_function(_new_name: Option<String>) {
    eprintln!("`pom project rename` - not implemented yet");
}

pub fn project_config_function() {
    eprintln!("`pom project config` - not implemented yet");
}

pub fn module_add_function(_module_name: Option<String>) {
    eprintln!("`pom module add` - not implemented yet");
}

pub fn module_rename_function(_old_module_name: Option<String>, _new_module_name: Option<String>) {
    eprintln!("`pom module rename` - not implemented yet");
}

pub fn module_remove_function(_module_name: Option<String>) {
    eprintln!("`pom module remove` - not implemented yet");
}

pub fn clean_function() {
    eprintln!("`pom clean` - not implemented yet");
}

pub fn build_function() {
    eprintln!("`pom build` - not implemented yet");
}

pub fn rebuild_function() {
    eprintln!("`pom rebuild` - not implemented yet");
}

pub fn flash_function() {
    eprintln!("`pom flash` - not implemented yet");
}

pub fn monitor_function() {
    eprintln!("`pom monitor` - not implemented yet");
}

pub fn config_function() {
    eprintln!("`pom config` - not implemented yet");
}

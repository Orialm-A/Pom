use crate::errors::{PomResult};



pub fn project_rename_function(_new_name: Option<String>) -> PomResult<()> {
    eprintln!("`pom project rename` - not implemented yet");
    Ok(())
}

pub fn project_config_function() -> PomResult<()> {
    eprintln!("`pom project config` - not implemented yet");
    Ok(())
}



pub fn module_rename_function(_old_module_name: Option<String>, _new_module_name: Option<String>) -> PomResult<()> {
    eprintln!("`pom module rename` - not implemented yet");
    Ok(())
}

pub fn module_remove_function(_module_name: Option<String>) -> PomResult<()> {
    eprintln!("`pom module remove` - not implemented yet");
    Ok(())
}

pub fn clean_function() -> PomResult<()> {
    eprintln!("`pom clean` - not implemented yet");
    Ok(())
}

pub fn build_function() -> PomResult<()> {
    eprintln!("`pom build` - not implemented yet");
    Ok(())
}

pub fn rebuild_function() -> PomResult<()> {
    eprintln!("`pom rebuild` - not implemented yet");
    Ok(())
}

pub fn flash_function() -> PomResult<()> {
    eprintln!("`pom flash` - not implemented yet");
    Ok(())
}

pub fn monitor_function() -> PomResult<()> {
    eprintln!("`pom monitor` - not implemented yet");
    Ok(())
}

pub fn config_function() -> PomResult<()> {
    eprintln!("`pom config` - not implemented yet");
    Ok(())
}

pub type PomResult<T> = Result<T, (PomErrorCode, Option<String>)>;

#[repr(i32)]  // `u8` more pertinent but would need a cast for `std::process::exit(code: i32)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomErrorCode {
    // Project Path errors: 1x
    ProjectPathEmpty = 10,
    ProjectPathInPomSource = 11,
    ProjectPathCurrentDirFailed = 12,
    ProjectPathDebugNotUnicode = 13,
    ProjectPathNotEmpty = 14,
    ProjectPathFailedToReadDir = 15,
    ProjectPathExistsAndNotDir = 16,
    ProjectPathFailedToCreateRoot = 17,
    // Config files errors: 2x
    ConfigFileDirTreePathNotFound = 20, // Not critical
    ConfigFileCantReadDirTree = 21,
    ConfigFileCantParseDirTree = 22,
    ConfigFileNoDirTreeSourcesFound = 23,
}

impl PomErrorCode {
    const fn exit_code(self) -> i32 {
        self as i32
    }

    const fn error_message(self) -> &'static str {
        match self {
            // Project Path errors: 1x
            PomErrorCode::ProjectPathEmpty => "Project path is empty or whitespace.",
            PomErrorCode::ProjectPathInPomSource => "Refusing to use this path as project root because it looks like Pom's source directory. Did you export `POM_DEV_TEST_PROJECT` correctly?",
            PomErrorCode::ProjectPathCurrentDirFailed => "Could not get current directory.",
            PomErrorCode::ProjectPathDebugNotUnicode => "Tried to create debug project in env var `POM_DEV_TEST_PROJECT` but the OS detected non-unicode characters.",
            PomErrorCode::ProjectPathNotEmpty => "Tried to create the project directory but it already exists and is not empty.",
            PomErrorCode::ProjectPathFailedToReadDir => "The specified project path exists but can't be read to check if it is empty.",
            PomErrorCode::ProjectPathExistsAndNotDir => "The specified project path exists but is a file.",
            PomErrorCode::ProjectPathFailedToCreateRoot => "Failed to create the project directory for OS reasons.",
            // Config files errors: 2x
            PomErrorCode::ConfigFileDirTreePathNotFound => "Internal: missing config source was handled as fatal. This is a Pom bug.",
            PomErrorCode::ConfigFileCantReadDirTree => "Found a dir tree config file but failed to read it.",
            PomErrorCode::ConfigFileCantParseDirTree => "Found a dir tree config file but failed to parse it.",
            PomErrorCode::ConfigFileNoDirTreeSourcesFound => "Did not found any dir tree config file.",
        }
    }

    pub fn handler(self, src: Option<&str>) -> ! {
        eprintln!("error[E{}]: {}", self.exit_code(), self.error_message());
        if let Some(details) = src {
            eprintln!("details: {}", details);
        }
        eprintln!("fatal - exiting pom now...");
        std::process::exit(self.exit_code());
    }
}

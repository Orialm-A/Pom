use owo_colors::OwoColorize;

pub type PomResult<T> = Result<T, (PomErrorCode, Option<String>)>;

#[repr(i32)]  // `u8` more pertinent but would need a cast for `std::process::exit(code: i32)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomErrorCode {
    // Path to project root errors: 1x
    PathToProjectRootEmpty = 10,
    PathToProjectRootInPomSource = 11,
    PathToProjectRootCantGetCurrentDir = 12,
    PathToProjectRootEnvVarNotUnicode = 13,
    PathToProjectRootNotEmpty = 14,
    PathToProjectRootFailedToReadDir = 15,
    PathToProjectRootExistsAndNotDir = 16,
    PathToProjectRootFailedToCreateRoot = 17,
    // Generation Layout file errors: 2x
    GenerationLayoutFileCandidateNotFound = 20, // Not critical
    GenerationLayoutFileCantRead = 21,
    GenerationLayoutFileCantParse = 22,
    GenerationLayoutFileCouldNotFindAny = 23,
    GenerationLayoutFileInvalidEntryPath = 24,
    // File system generation errors: 3x
    ProjectGenerationFailedToCreateSubDir = 30,
    ProjectGenerationFailedToCreateDocGroups = 31,
    ProjectGenerationGroupsFileWriteFailed = 32,
}

impl PomErrorCode {
    const fn exit_code(self) -> i32 {
        self as i32
    }

    const fn error_message(self) -> &'static str {
        match self {
            // Path to project root errors: 1x
            PomErrorCode::PathToProjectRootEmpty => "Project path is empty or whitespace.",
            PomErrorCode::PathToProjectRootInPomSource => "Refusing to use this path as project root because it looks like Pom's source directory. Did you export `POM_DEV_TEST_PROJECT` correctly?",
            PomErrorCode::PathToProjectRootCantGetCurrentDir => "Could not get current directory.",
            PomErrorCode::PathToProjectRootEnvVarNotUnicode => "Tried to create debug project in env var `POM_DEV_TEST_PROJECT` but the OS detected non-unicode characters.",
            PomErrorCode::PathToProjectRootNotEmpty => "Tried to create the project directory but it already exists and is not empty.",
            PomErrorCode::PathToProjectRootFailedToReadDir => "The specified project path exists but can't be read to check if it is empty.",
            PomErrorCode::PathToProjectRootExistsAndNotDir => "The specified project path exists but is a file.",
            PomErrorCode::PathToProjectRootFailedToCreateRoot => "Failed to create the project directory for OS reasons.",
            // Config files errors: 2x
            PomErrorCode::GenerationLayoutFileCantRead => "Found a generation layout file but failed to read it.",
            PomErrorCode::GenerationLayoutFileCantParse => "Found a dir tree config file but failed to parse it.",
            PomErrorCode::GenerationLayoutFileCouldNotFindAny => "Did not found any dir tree config file.",
            PomErrorCode::GenerationLayoutFileInvalidEntryPath => "The field `path` of a generation layout entry is invalid, failed to extract its name.",
            // File system generation errors: 3x
            PomErrorCode::ProjectGenerationFailedToCreateSubDir => "Failed to create a subdir for the project.",
            PomErrorCode::ProjectGenerationFailedToCreateDocGroups => "Failed to create `doc_groups.h`.",
            PomErrorCode::ProjectGenerationGroupsFileWriteFailed => "Successfully created `doc_groups.h` but failed to fill it.",
            _ => "Internal: missing config source was handled as fatal. This is a Pom bug.",

        }
    }

    pub fn handler(self, src: Option<&str>) -> ! {
        eprintln!(
            "{}: {}",
            format!("error [E{}]", self.exit_code()).red().bold(),
            self.error_message().bold()
        );

        if let Some(details) = src {
            eprintln!("{}", format!("details: {}", details).red());
        }

        eprintln!("{}", "fatal - exiting pom now...".red());
        std::process::exit(self.exit_code());
    }
}

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
    // Filesystem errors: 3x            // refac done
    FilesystemDirCreationFail = 30,
    FilesystemEntryInvalid = 31,
    FilesystemStripPathPrefixFail = 32,
    FilesystemUnsupportedEntryType = 33,
    FilesystemCopySourceMissing = 34,
    FilesystemCopySourceNotDir = 35,
    FilesystemCopyDestExists = 36,
    FilesystemCopyFail = 37,
    // File creation errors: 4x
    FileCreationFail = 40,
    FileWriteFail = 41,
    // `pom.toml` file errors: 5x
    PomTomlFileSerializationFail = 50,
    // Target-free files errors: 6x
    TargetFreeFilesDefaultSourceMissing = 60,
    TargetFreeFilesSourceReadFail = 61,     // A
    TargetFreeFilesInvalidEntry = 62,       // B - Entries when exploring dir (recur. or not)
    TargetFreeFilesAlreadyExists = 63,      // Replaced by 36
    TargetFreeFilesCopyFail = 64,           // Replaced by 37
    TargetFreeFilesDefaultSourceEmpty = 65, // C
    // Target files errors: 7x
    TargetSelectionFail = 70,
    TargetFilesSourceReadFail = 71,         // A
    TargetFilesInvalidEntry = 72,           // B - Has nothing to do with 24 but same name :/
    TargetFilesDefaultSourceEmpty = 75,     // C


                                            // A, B, C  -> Refactor directory access/walking
                                            // D, F     -> Refactor file copy
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
            // subdirectories errors: 3x
            PomErrorCode::FilesystemDirCreationFail => "Failed to create a subdir for the project.",
            // File creation errors: 4x
            PomErrorCode::FileCreationFail => "Failed to create a file.",
            PomErrorCode::FileWriteFail => "Successfully created a file but failed to fill it.",
            // `pom.toml` file errors: 5x
            PomErrorCode::PomTomlFileSerializationFail => "Failed to serialize project settings for `pom.toml`",
            // Target-free files errors: 6x
            PomErrorCode::TargetFreeFilesDefaultSourceMissing => "The default source for target-free files is missing.",
            PomErrorCode::TargetFreeFilesSourceReadFail => "Can't read content in target-free files source directory.",
            PomErrorCode::TargetFreeFilesInvalidEntry => "Found an invalid entry in target-free files source directory.",
            PomErrorCode::TargetFreeFilesAlreadyExists => "Tried to create a file that already exists",
            PomErrorCode::TargetFreeFilesCopyFail => "Failed to copy a file.",
            PomErrorCode::TargetFreeFilesDefaultSourceEmpty => "The default source for target-free files is empty.",
            // Target files errors: 7x
            PomErrorCode::TargetSelectionFail => "An error occured when selecting the target.",
            PomErrorCode::TargetFilesSourceReadFail => "Can't read content in target files source directory.",
            PomErrorCode::TargetFilesInvalidEntry => "Found an invalid entry in target files source directory.",
            PomErrorCode::TargetFilesDefaultSourceEmpty => "The default source for target files is empty.",
            // Non fatal errors fallback
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

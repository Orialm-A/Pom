//! Errors module
//!
//! This module centralize all the possible error codes and the handler.
//! Errors can be escalated up to `main()` where they'll be displayed and trigger `std::process::exit`.
//! They cal also be catched anywhere before `main()`.

use owo_colors::OwoColorize;

/// Return type for functions
///
/// Allows to return either a result of any type (`<T>`), or an error tuple containing a `PomErrorCode` and
/// an optional String for details.
pub type PomResult<T> = Result<T, (PomErrorCode, Option<String>)>;

#[repr(i32)]  // `u8` more pertinent but would need a cast for `std::process::exit(code: i32)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents all the error codes
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
    GenerationLayoutFileCantOpen = 21,
    GenerationLayoutFileCantRead = 22,
    GenerationLayoutFileCouldNotFindAny = 23,
    GenerationLayoutFileInvalidEntryPath = 24,
    // Filesystem errors: 3xx
    FilesystemDirCreationFail = 300,
    FilesystemFileCreationFail = 301,
    FilesystemFileWriteFail = 302,
    FilesystemFileReadFail = 303,
    FilesystemEntryInvalid = 304,
    FilesystemStripPathPrefixFail = 305,
    FilesystemUnsupportedEntryType = 306,
    FilesystemCopySourceMissing = 307,
    FilesystemCopySourceNotDir = 308,
    FilesystemCopyFail = 309,
    FilesystemFileOverwriteForbidded = 310,
    // Prompt errors: 4x
    PromptSelectionFail = 40,
    PromptStringFail = 41,
    // `pom.toml` file errors: 5x
    PomTomlFileSerializationFail = 50,
    PomTomlFileDeserializationFail = 51,
    PomTomlNotFound = 52,
    PomTomlNotFile = 53,
    PomTomlFileCantOpen = 54,
    // Assets errors: 6x
    FileTemplateMissing = 60,
    // Module templates: 7x
    ModuleTemplateSourceNotFound = 70,
    ModuleTemplateSourceNotDir = 71,
    ModuleTemplateCouldNotFindAny = 72,
    ModuleTemplateMissing = 73,
    ModuleTemplateUnexpectedContentFound = 74,

}

impl PomErrorCode {
    const fn exit_code(self) -> i32 {
        self as i32
    }

    const fn error_message(self) -> &'static str {
        match self {
            Self::PathToProjectRootEmpty => "Project path is empty or whitespace.",
            Self::PathToProjectRootInPomSource => "Project path is in Pom's source directory. Export `POM_DEV_TEST_PROJECT`.",
            Self::PathToProjectRootCantGetCurrentDir => "Could not get the current directory.",
            Self::PathToProjectRootEnvVarNotUnicode => "Env var `POM_DEV_TEST_PROJECT` contains non-Unicode characters.",
            Self::PathToProjectRootNotEmpty => "Project path already exists and is not empty.",
            Self::PathToProjectRootFailedToReadDir => "Project path exists but could not be read.",
            Self::PathToProjectRootExistsAndNotDir => "Project path exists but is not a directory.",
            Self::PathToProjectRootFailedToCreateRoot => "Failed to create the project root due to an OS error.",

            // Generation layout file errors
            Self::GenerationLayoutFileCantOpen => "Generation layout file was found but could not be opened.",
            Self::GenerationLayoutFileCantRead => "Generation layout file was found but could not be read.",
            Self::GenerationLayoutFileCouldNotFindAny => "Generation layout file was not found.",
            Self::GenerationLayoutFileInvalidEntryPath => "Generation layout file has an invalid `path` field.",

            // Filesystem errors
            Self::FilesystemDirCreationFail => "Failed to create directory.",
            Self::FilesystemFileCreationFail => "Failed to create file.",
            Self::FilesystemFileWriteFail => "Failed to write file.",
            Self::FilesystemFileReadFail => "Failed to read file.",
            Self::FilesystemEntryInvalid => "Invalid directory entry found.",
            Self::FilesystemStripPathPrefixFail => "Failed to strip path prefix.",
            Self::FilesystemUnsupportedEntryType => "Unsupported filesystem entry type found.",
            Self::FilesystemCopySourceMissing => "Copy source is missing.",
            Self::FilesystemCopySourceNotDir => "Copy source is not a directory.",
            Self::FilesystemCopyFail => "Copy failed.",
            Self::FilesystemFileOverwriteForbidded => "File copy or creation failed because overwrite is forbidden by policy.",

            // Prompt errors
            Self::PromptSelectionFail => "Failed to prompt for menu item selection.",
            Self::PromptStringFail => "Failed to prompt for input.",

            // pom.toml
            Self::PomTomlFileSerializationFail => "Failed to serialize `pom.toml` content.",
            Self::PomTomlFileDeserializationFail => "Failed to deserialize `pom.toml` content.",
            Self::PomTomlNotFound => "Can't found `pom.toml`.",
            Self::PomTomlNotFile => "`pom.toml` found but is not a file.",
            Self::PomTomlFileCantOpen => "`pom.toml` was found but could not be opened.",

            // Templates/assets
            Self::FileTemplateMissing => "File templates are missing from the default assets.",

            //
            Self::ModuleTemplateSourceNotDir => "Module tmplate source is not a directory.",
            Self::ModuleTemplateCouldNotFindAny => "Module template source was not found.",
            Self::ModuleTemplateMissing => "Module template is missing.",
            Self::ModuleTemplateUnexpectedContentFound => "Expected to find only `template.c.pomrt` and `template.h.pomrt` in module template source.",

            // Fallback
            _ => "Internal: A non-critical error was handled as fatal. This is a Pom bug.",

        }
    }

    /// Final error handler
    ///
    /// Will print the base error message, and the option details if received.
    /// Exit the program.
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

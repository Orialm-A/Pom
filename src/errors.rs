pub type PomResult<T> = Result<T, (PomErrorCode, Option<String>)>;

#[repr(i32)]  // `u8` more pertinent but would need a cast for `std::process::exit(code: i32)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PomErrorCode {
    // Project Path errors: 1x
    ProjectPathEmpty = 10,
    ProjectPathInPomSource = 11,
    ProjectPathCurrentDirFailed = 12,
    ProjectPathDebugNotUnicode = 13,
}

impl PomErrorCode {
    const fn exit_code(self) -> i32 {
        self as i32
    }

    const fn error_message(self) -> &'static str {
        match self {
            PomErrorCode::ProjectPathEmpty => "Project path is empty or whitespace.",
            PomErrorCode::ProjectPathInPomSource => "Refusing to use this path as project root because it looks like Pom's source directory. Did you export `POM_DEV_TEST_PROJECT` correctly?",
            PomErrorCode::ProjectPathCurrentDirFailed => "Could not get current directory.",
            PomErrorCode::ProjectPathDebugNotUnicode => "Tried to create debug project in env var `POM_DEV_TEST_PROJECT` but the OS detected non-unicode characters.",
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

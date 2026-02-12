use std::collections::HashMap;
use std::path::{PathBuf, Path};
use regex::Regex;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKey {
    ProjectName,
    ProjectNameNormalized,
}


impl FieldKey {
    pub fn from_str(candidate: &str) -> Option<Self> {
        Some(match candidate {
            "project_name" => FieldKey::ProjectName,
            "project_name_normalized" => FieldKey::ProjectNameNormalized,
            _ => return None,
        })
    }
}


pub struct TemplateFields {
    values: HashMap<FieldKey, String>,
}


impl TemplateFields {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    pub fn insert(&mut self, key: FieldKey, value: impl Into<String>) {
        self.values.insert(key, value.into());
    }

    pub fn get(&self, key: FieldKey) -> Option<&str> {
        self.values.get(&key).map(|s| s.as_str())
    }

    /// Used by the renderer after extracting `{{...}}` from a template
    pub fn get_by_str(&self, key: &str) -> Option<&str> {
        FieldKey::from_str(key).and_then(|k| self.get(k))
    }
}


pub fn get_rendered_template_dest(file_path: &Path) -> Option<PathBuf> {
    // Check extension
    if file_path.extension()? != "pomrt" {
        return None;
    }

    // Remove the `.pomrt` extension
    let mut stripped = file_path.to_path_buf();
    stripped.set_extension("");

    Some(stripped)
}


pub fn render_template(template_content: &str, fields: &TemplateFields) -> String {
    // With Regex crate, Putting parenthesis around a litteral make it a group in the captured
    // string, accessible in an iterator starting at 1. (0 is for the full captured string.)
    // `?P<something>` names this group `something`, and it can be used to access the group
    // instead of numbers. Using it there is overzealous but that's a simple case so perfect
    // example

    let regular_expression = Regex::new(r"(?x) # extended mode: Ignores whitespace / allow comments
                                        \{\{pom:  # Start of the pattern identification
                                        (?P<field_name>[a-zA-Z_]+) # What must be extracted
                                        \}\}      # End of the the pattern identification
    ").unwrap();
    // Returns Result<Regex, regex::Error>. The expression is hardcoded and without user input, so
    // no need to catch the `regex::Error` to fire a `PomErrorCode`. I see issues at compile time.

    regular_expression.replace_all(template_content, |captured: &regex::Captures| {
        let key = &captured["field_name"];
        fields.get_by_str(key)
        .map(|s| s.to_owned())
        .unwrap_or(captured.get(0).unwrap().as_str().to_owned())
        // Key string equivalent or default to full experession if not available

    })
    .into_owned()
}

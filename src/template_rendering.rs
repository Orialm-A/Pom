use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKey {
    ProjectName,
    ProjectNameSnakeCase,
}


impl FieldKey {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldKey::ProjectName => "project_name",  // This is counter intuitive, it means the name with emoji
            FieldKey::ProjectNameSnakeCase => "project_name_snake_case",
        }
    }

    pub fn from_str(candidate: &str) -> Option<Self> {
        Some(match candidate {
            "project_name" => FieldKey::ProjectName,
            "project_name_snake_case" => FieldKey::ProjectNameSnakeCase,
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

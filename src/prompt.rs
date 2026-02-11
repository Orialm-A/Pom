//! Prompt module
//!
//! Prompt user for missing information

use unicode_normalization::UnicodeNormalization;
use dialoguer::{Select, Input, theme::ColorfulTheme};
use crate::errors::{PomErrorCode, PomResult};
use std::collections::HashMap;
use std::path::{PathBuf,};



/// Prompt the user for a string if the passed one is `None`
///
/// May error `PomErrorCode::PromptStringFail`
pub fn prompt_if_missing_string(optional_string: Option<String>, prompt_hint: &str) -> PomResult<String> {
    match optional_string {
        Some(optional_string) => Ok(optional_string),
        None => {
            let prompted_string: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt_hint)
                .interact_text()
                .map_err(|e| (PomErrorCode::PromptStringFail, Some(e.to_string())))?;
            Ok(prompted_string)
        }
    }
}


/// Convert a string into a snake_case, alpha-numeric only slug
///
/// Replaces accentuated letters with non-accentuated equivalent
/// Replace punctuations with `_`
/// Removes other characters (emoji, sharp...)
pub fn slugify_snake(input: &str) -> String {
    let mut normalized_string = String::new();
    let mut last_was_underscore = false;

    for character in input.nfkd() {
        if('\u{0300}'..='\u{036F}').contains(&character) {
            continue;
        }

        if character.is_ascii_alphanumeric() {
            normalized_string.push(character.to_ascii_lowercase());
            last_was_underscore = false;
        } else if matches!(character, ' ' | '-' | '_' | '.' | ',' | ';' | ':' | '/') {
            if !normalized_string.is_empty() && !last_was_underscore {
                normalized_string.push('_');
                last_was_underscore = true
            }
        } else {
            // Ignore any other character like emoji
        }
    }

    while normalized_string.ends_with('_') {
        normalized_string.pop();
    }

    normalized_string
}


/// Select a target from a menu
pub fn select_target(available_targets: &HashMap<String, PathBuf>) -> PomResult<PathBuf> {
    let mut keys: Vec<&String> = available_targets.keys().collect();

    if keys.is_empty() {
        return Err((PomErrorCode::FileTemplateMissing, Some("In `assets/target`".to_string())));
    }

    keys.sort();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Target not found. Select one of the available targets")
        .items(&keys)
        .default(0)
        .interact()
        .map_err(|e| (
            PomErrorCode::PromptTargetSelectionFail,
            Some(e.to_string()),
        ))?;


    let selected_key = keys[selection];
    Ok(available_targets[selected_key].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_lowercase_and_spaces() {
        assert_eq!(slugify_snake("Hello World"), "hello_world");
    }

    #[test]
    fn collapses_separators() {
        assert_eq!(slugify_snake("Hello---World"), "hello_world");
        assert_eq!(slugify_snake("Hello   World"), "hello_world");
        assert_eq!(slugify_snake("a..b"), "a_b");
    }

    #[test]
    fn strips_diacritics() {
        assert_eq!(slugify_snake("Île de France"), "ile_de_france");
        assert_eq!(slugify_snake("café"), "cafe");
    }

    #[test]
    fn drops_weird_chars() {
        assert_eq!(slugify_snake("my:proj^ect!"), "my_project");
    }

    #[test]
    fn drop_emoji() {
        assert_eq!(slugify_snake("🚀 my_awesome_project 🚀"), "my_awesome_project");
    }

    #[test]
    fn trims_underscores() {
        assert_eq!(slugify_snake("  hello  "), "hello");
        assert_eq!(slugify_snake("---hello---"), "hello");
    }

    #[test]
    fn complex_real_world_input() {
        assert_eq!(
            slugify_snake("my:proj/is-awesome (3)   🚀!!!--  "),
            "my_proj_is_awesome_3"
        );
    }
}

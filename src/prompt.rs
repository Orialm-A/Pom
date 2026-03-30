//! User prompts.
//!
//! Provides all the user to get user input at runtime.

use crate::errors::{PomErrorCode, PomResult};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use unicode_normalization::UnicodeNormalization;

/// Prompt the user for a string if the passed one is `None`.
///
/// # Arguments
/// - `optional_string` - The string that may be missing
/// - `prompt_hint` - Prompt shown to the user
/// - `empty_string_allowed` - Allow the user to continue without typing anything
///
/// # Returns
/// The string wrapped if `optional_string` if available, or the one typed by the user.
///
/// # Errors
/// - `PomErrorCode::PromptStringFail` if the prompt fails due to an OS error
pub fn prompt_if_missing_string(
    optional_string: Option<String>,
    prompt_hint: &str,
    empty_string_allowed: bool,
) -> PomResult<String> {
    match optional_string {
        Some(optional_string) => Ok(optional_string),
        None => {
            let prompted_string: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt_hint)
                .allow_empty(empty_string_allowed)
                .interact_text()
                .map_err(|e| (PomErrorCode::PromptStringFail, Some(e.to_string())))?;
            Ok(prompted_string)
        }
    }
}

/// Convert a string into a snake_case slug.
///
/// The resulting slug:
/// - uses snake_case
/// - replaces accented letters (e.g. `é` → `e`)
/// - replaces punctuation with `_`
/// - removes unsupported special characters
/// - collapses consecutive `_` into a single `_`
/// - trims leading and trailing `_`
///
/// # Arguments
/// - `input` - The text to slugify; It is not modified in place
pub fn slugify_snake(input: &str) -> String {
    let mut normalized_string = String::new();
    let mut last_was_underscore = false;

    for character in input.nfkd() {
        if ('\u{0300}'..='\u{036F}').contains(&character) {
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

/// Prompt the user to select an entry in a menu.
///
/// This function order the entries alphabetically
///
/// # Arguments
/// - `items` - Collection of menu entries
/// - `prompt_hint` - Prompt shown to the user
///
/// # Returns
/// The selected entry.
///
/// # Errors
/// - `PomErrorCode::PromptSelectionFail` if the prompt fails due to an OS error
pub fn select(items: &[String], prompt_hint: &str) -> PomResult<String> {
    let mut items = items.to_vec();
    items.sort();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt_hint)
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| (PomErrorCode::PromptSelectionFail, Some(e.to_string())))?;

    Ok(items[selection].clone())
}

/// Prompt the user for confirmation.
///
/// # Arguments
/// - `prompt_hint` - Prompt shown to the user
///
/// # Errors
/// - `PomErrorCode::PromptSelectionFail` if the prompt fails due to an OS error
pub fn confirm(prompt_hint: &str) -> PomResult<bool> {
    let result = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt_hint)
        .interact() // Returns `Result<bool>`
        .map_err(|e| (PomErrorCode::PromptConfirmationFail, Some(e.to_string())))?;

    Ok(result)
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
        assert_eq!(
            slugify_snake("🚀 my_awesome_project 🚀"),
            "my_awesome_project"
        );
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

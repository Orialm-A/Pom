//! User prompts.
//!
//! Provides all the user to get user input at runtime.

use crate::errors::{PomErrorCode, PomResult};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

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
pub fn input_text(
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

//! Text formatting.
//!
//! Utilities for transforming user-provided names into code-friendly identifiers.

use convert_case::{Case, Casing};
use unicode_normalization::UnicodeNormalization;

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
pub fn get_slug(input: &str) -> String {
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

/// Convert a string to `CONSTANT_CASE`.
///
/// # Arguments
/// - `input` - Text to convert
///
/// # Returns
/// The input converted to `CONSTANT_CASE`.
pub fn get_const_case(input: &str) -> String {
    input.to_case(Case::Constant)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod string_slugifying_tests {
        use super::*;

        #[test]
        fn basic_lowercase_and_spaces() {
            assert_eq!(get_slug("Hello World"), "hello_world");
        }

        #[test]
        fn collapses_separators() {
            assert_eq!(get_slug("Hello---World"), "hello_world");
            assert_eq!(get_slug("Hello   World"), "hello_world");
            assert_eq!(get_slug("a..b"), "a_b");
        }

        #[test]
        fn strips_diacritics() {
            assert_eq!(get_slug("Île de France"), "ile_de_france");
            assert_eq!(get_slug("café"), "cafe");
        }

        #[test]
        fn drops_weird_chars() {
            assert_eq!(get_slug("my:proj^ect!"), "my_project");
        }

        #[test]
        fn drop_emoji() {
            assert_eq!(get_slug("🚀 my_awesome_project 🚀"), "my_awesome_project");
        }

        #[test]
        fn trims_underscores() {
            assert_eq!(get_slug("  hello  "), "hello");
            assert_eq!(get_slug("---hello---"), "hello");
        }

        #[test]
        fn complex_real_world_input() {
            assert_eq!(
                get_slug("my:proj/is-awesome (3)   🚀!!!--  "),
                "my_proj_is_awesome_3"
            );
        }
    }

    mod string_const_case_tests {
        use super::*;

        #[test]
        fn convert_snake_case_to_const_case() {
            assert_eq!(get_const_case("hello_world"), "HELLO_WORLD");
        }

        #[test]
        fn convert_sentence_to_const_case() {
            assert_eq!(get_const_case("hello world"), "HELLO_WORLD");
        }

        #[test]
        fn preserve_already_const_case() {
            assert_eq!(get_const_case("HELLO_WORLD"), "HELLO_WORLD");
        }

        #[test]
        fn convert_empty_string_to_const_case() {
            assert_eq!(get_const_case(""), "");
        }
    }
}

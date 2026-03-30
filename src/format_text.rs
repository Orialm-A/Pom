//! Text formatting.
//!
//! Utilities for transforming user-provided names into code-friendly identifiers.

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

#[cfg(test)]
mod tests {
    use super::*;

    mod string_slugifying_tests {
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
}

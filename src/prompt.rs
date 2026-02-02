use text_io::read;
use unicode_normalization::UnicodeNormalization;

pub fn prompt_if_missing_string(optional: Option<String>, prompt_hint: &str) -> String {
    match optional {
        Some(extracted_string) => { extracted_string },
        None => {
            print!("{}: ", prompt_hint);
            read!("{}\n") // From text_io
        }
    }
}

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
        } else if matches!(character, ' ' | '-' | '_' | '.' | ':' | '/') {
            if !normalized_string.is_empty() && !last_was_underscore {
                normalized_string.push('_');
                last_was_underscore = true
            }
        } else {
            // Ignore any other character like colon or emoji
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

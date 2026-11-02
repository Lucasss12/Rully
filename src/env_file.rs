use crate::errors::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Reads an environment file from disk and parses its variables.
pub fn read(path: &Path) -> Result<HashMap<String, String>, AppError> {
    let source = fs::read_to_string(path).map_err(AppError::FileRead)?;
    parse(&source)
}

/// Parses a dotenv-style source into `name -> value` variables.
///
/// Lines are `KEY=value`, `#` starts a comment, blank lines are ignored, and
/// surrounding quotes are stripped from values. When a key repeats, the last
/// value wins. A line without `=` or with an empty key is an error.
pub fn parse(source: &str) -> Result<HashMap<String, String>, AppError> {
    let mut variables = HashMap::new();

    for (index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (name, value) = line.split_once('=').ok_or_else(|| {
            // The line itself is deliberately not echoed: a malformed line
            // may hold a secret, and error output ends up in logs.
            AppError::InvalidEnvFile(format!("invalid environment line {}", index + 1))
        })?;

        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::InvalidEnvFile(format!(
                "missing variable name in line {}",
                index + 1
            )));
        }

        variables.insert(name.to_string(), unquote(value.trim()));
    }

    Ok(variables)
}

/// Removes one pair of matching surrounding quotes, if any.
fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if value.len() >= 2
        && (bytes[0] == b'"' || bytes[0] == b'\'')
        && bytes[bytes.len() - 1] == bytes[0]
    {
        return value[1..value.len() - 1].to_string();
    }
    value.to_string()
}

/// Replaces every `{{name}}` placeholder with its value from `vars`.
///
/// Names are trimmed (`{{ base_url }}` matches `base_url`). Text without
/// placeholders is returned unchanged, and an unclosed `{{` is kept verbatim.
/// A placeholder whose name is not in `vars` is an error naming the variable —
/// the value itself never appears in the message. Inserted values are not
/// rescanned, so a value containing `{{` stays literal.
pub fn interpolate(text: &str, vars: &HashMap<String, String>) -> Result<String, AppError> {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(open) = rest.find("{{") {
        result.push_str(&rest[..open]);
        let after = &rest[open + 2..];

        let Some(close) = after.find("}}") else {
            result.push_str(&rest[open..]);
            return Ok(result);
        };

        let name = after[..close].trim();
        let value = vars
            .get(name)
            .ok_or_else(|| AppError::UnknownVariable(name.to_string()))?;
        result.push_str(value);
        rest = &after[close + 2..];
    }

    result.push_str(rest);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_key_value_pairs() {
        let source = "# comment\n\
                      \n\
                      base_url=https://api.dev.example.com\n\
                      api_key=abc123\n";

        let variables = parse(source).unwrap();

        assert_eq!(variables["base_url"], "https://api.dev.example.com");
        assert_eq!(variables["api_key"], "abc123");
    }

    #[test]
    fn trims_whitespace_around_keys_and_values() {
        let variables = parse("  base_url  =  https://api.example.com  ").unwrap();

        assert_eq!(variables["base_url"], "https://api.example.com");
    }

    #[test]
    fn strips_surrounding_quotes() {
        let variables = parse("double=\"hello world\"\nsingle='hello world'\n").unwrap();

        assert_eq!(variables["double"], "hello world");
        assert_eq!(variables["single"], "hello world");
    }

    #[test]
    fn keeps_unmatched_quotes_in_values() {
        let variables = parse("quote=\"unbalanced\n").unwrap();

        assert_eq!(variables["quote"], "\"unbalanced");
    }

    #[test]
    fn last_value_wins_on_duplicate() {
        let variables = parse("env=dev\nenv=prod\n").unwrap();

        assert_eq!(variables["env"], "prod");
    }

    #[test]
    fn rejects_a_line_without_a_separator() {
        let result = parse("base_url\n");

        assert!(matches!(result, Err(AppError::InvalidEnvFile(_))));
    }

    #[test]
    fn parse_errors_never_echo_the_line_content() {
        // A malformed line may hold a pasted secret: only the line number
        // may appear in the message.
        let error = parse("# ok\napi_key super-secret-value").unwrap_err();

        let message = error.to_string();
        assert!(message.contains("line 2"), "{message}");
        assert!(!message.contains("super-secret-value"), "{message}");
    }

    #[test]
    fn rejects_an_empty_variable_name() {
        let result = parse("=value\n");

        assert!(matches!(result, Err(AppError::InvalidEnvFile(_))));
    }

    #[test]
    fn parses_an_empty_source() {
        let variables = parse("# only a comment\n\n").unwrap();

        assert!(variables.is_empty());
    }

    #[test]
    fn reads_variables_from_a_file() {
        let dir = std::env::temp_dir().join("rully-env-file-read");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dev.env");
        fs::write(&path, "base_url=https://api.dev.example.com\n").unwrap();

        let variables = read(&path).unwrap();

        assert_eq!(variables["base_url"], "https://api.dev.example.com");
        let _ = fs::remove_dir_all(&dir);
    }

    fn vars() -> HashMap<String, String> {
        HashMap::from([
            (
                "base_url".to_string(),
                "https://api.dev.example.com".to_string(),
            ),
            ("token".to_string(), "abc123".to_string()),
        ])
    }

    #[test]
    fn interpolates_variables_into_text() {
        assert_eq!(
            interpolate("{{base_url}}/users", &vars()).unwrap(),
            "https://api.dev.example.com/users"
        );
    }

    #[test]
    fn interpolates_multiple_variables_in_one_text() {
        assert_eq!(
            interpolate("{{base_url}}/x?token={{token}}", &vars()).unwrap(),
            "https://api.dev.example.com/x?token=abc123"
        );
    }

    #[test]
    fn leaves_text_without_placeholders_unchanged() {
        assert_eq!(
            interpolate("https://api.example.com/users", &vars()).unwrap(),
            "https://api.example.com/users"
        );
    }

    #[test]
    fn trims_whitespace_inside_placeholders() {
        assert_eq!(
            interpolate("{{ base_url }}", &vars()).unwrap(),
            "https://api.dev.example.com"
        );
    }

    #[test]
    fn reports_the_missing_variable_name_without_its_value() {
        let error = interpolate("{{secret}}", &vars()).unwrap_err();

        let message = error.to_string();
        assert!(message.contains("secret"), "{message}");
        assert!(matches!(error, AppError::UnknownVariable(name) if name == "secret"));
    }

    #[test]
    fn keeps_an_unclosed_placeholder_verbatim() {
        assert_eq!(interpolate("{{base_url", &vars()).unwrap(), "{{base_url");
    }

    #[test]
    fn does_not_rescan_inserted_values() {
        let vars = HashMap::from([("literal".to_string(), "{{token}}".to_string())]);

        assert_eq!(interpolate("{{literal}}", &vars).unwrap(), "{{token}}");
    }
}

use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq)]
pub(crate) enum Method {
    #[value(name = "GET")]
    Get,
    #[value(name = "POST")]
    Post,
    #[value(name = "PUT")]
    Put,
    #[value(name = "PATCH")]
    Patch,
    #[value(name = "DELETE")]
    Delete,
}

#[derive(Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    pub(crate) method: Method,
    pub(crate) url: String,

    #[arg(long = "header")]
    pub(crate) headers: Vec<String>,

    #[arg(long = "query")]
    pub(crate) queries: Vec<String>,

    #[arg(long = "body")]
    pub(crate) body: Option<String>,
    
    #[arg(long = "body-file")]
    pub(crate) body_file: Option<String>,

    #[arg(short, long)]
    pub(crate) verbose: bool,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_missing_required_arguments() {
        let result = Cli::try_parse_from(["rully"]);
        assert!(result.is_err());
    }
    
    #[test]
    fn rejects_missing_url() {
        let result = Cli::try_parse_from(["rully", "GET"]);
        assert!(result.is_err());
    }
    
    #[test]
    fn rejects_invalid_method() {
        let result = Cli::try_parse_from([
            "rully",
            "https://example.com",
        ]);
        assert!(result.is_err());
    }
    
    #[test]
    fn rejects_unknown_method() {
        let result = Cli::try_parse_from([
            "rully",
            "OPTIONS",
            "https://example.com",
        ]);
        assert!(result.is_err());
    }
    
    #[test]
    fn parses_default_arguments() {
        let cli = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
        ])
        .unwrap();
        assert_eq!(cli.method, Method::Get);
        assert_eq!(cli.url, "https://example.com");
        assert!(cli.headers.is_empty());
        assert!(cli.queries.is_empty());
        assert!(cli.body.is_none());
        assert!(!cli.verbose);
    }
    
    #[test]
    fn parses_all_supported_methods() {
        let methods = [
            ("GET", Method::Get),
            ("POST", Method::Post),
            ("PUT", Method::Put),
            ("PATCH", Method::Patch),
            ("DELETE", Method::Delete),
        ];
        for (input, expected) in methods {
            let cli = Cli::try_parse_from([
                "rully",
                input,
                "https://example.com",
            ])
            .unwrap();
            assert_eq!(cli.method, expected);
        }
    }
    
    #[test]
    fn parses_multiple_headers() {
        let cli = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Authorization: Bearer token",
            "--header",
            "Content-Type: application/json",
        ])
        .unwrap();
        assert_eq!(cli.headers.len(), 2);
        assert_eq!(cli.headers[0], "Authorization: Bearer token");
        assert_eq!(cli.headers[1], "Content-Type: application/json");
    }
    
    #[test]
    fn parses_multiple_queries() {
        let cli = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--query",
            "page=1",
            "--query",
            "limit=10",
        ])
        .unwrap();
        assert_eq!(cli.queries, vec!["page=1", "limit=10"]);
    }
    
    #[test]
    fn parses_body() {
        let cli = Cli::try_parse_from([
            "rully",
            "POST",
            "https://example.com",
            "--body",
            r#"{"name":"John"}"#,
        ])
        .unwrap();
        assert_eq!(cli.body.as_deref(), Some(r#"{"name":"John"}"#));
    }
    
    #[test]
    fn parses_verbose_flags() {
        let long = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--verbose",
        ])
        .unwrap();
        let short = Cli::try_parse_from([
            "rully",
            "GET",
            "https://example.com",
            "-v",
        ])
        .unwrap();
        assert!(long.verbose);
        assert!(short.verbose);
    }
}
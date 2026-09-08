use clap::{Parser, ValueEnum};
use std::time::Instant;

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq)]
enum Method {
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
struct Cli {
    method: Method,
    url: String,

    #[arg(long = "header")]
    headers: Vec<String>,

    #[arg(long = "query")]
    queries: Vec<String>,

    #[arg(long = "body")]
    body: Option<String>,

    #[arg(short, long)]
    verbose: bool,
}

fn format_size(bytes: usize) -> String {
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < units.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", units[unit])
    } else {
        format!("{size:.1} {}", units[unit])
    }
}

fn is_sensitive_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("authorization")
        || name.eq_ignore_ascii_case("cookie")
        || name.eq_ignore_ascii_case("set-cookie")
        || name.eq_ignore_ascii_case("proxy-authorization")
        || name.eq_ignore_ascii_case("x-api-key")
}

fn display_header_value(name: &str, value: &str) -> String {
    if is_sensitive_header(name) {
        "[redacted]".to_string()
    } else {
        value.to_string()
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let start = Instant::now();

    let client = reqwest::Client::new();

    let method = match cli.method {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
    };

    let url = match reqwest::Url::parse(&cli.url) {
        Ok(url) => url,
        Err(error) => {
            eprintln!("URL invalide : {error}");
            return;
        }
    };

    let mut queries = Vec::new();
    for query in &cli.queries {
        let (name, value) = match query.split_once('=') {
            Some(pair) => pair,
            None => {
                eprintln!("Format attendu pour --query : nom=valeur");
                return;
            }
        };
        if url
            .query_pairs()
            .any(|(existing_name, _)| existing_name == name)
        {
            eprintln!("Paramètre déjà présent dans l'URL : {name}");
            return;
        }
        queries.push((name.to_string(), value.to_string()));
    }

    let mut request = client.request(method, url);
    for header in &cli.headers {
        let (name, value) = match header.split_once(':') {
            Some(pair) => pair,
            None => {
                eprintln!("Format attendu pour --header : Nom: valeur");
                return;
            }
        };
        request = request.header(name.trim(), value.trim());

        if cli.verbose {
            println!(
                "→ Header: {}: {}",
                name.trim(),
                display_header_value(name.trim(), value.trim())
            );
        }
    }

    if let Some(body) = cli.body {
        request = request.body(body)
    }

    request = request.query(&queries);

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Erreur réseau : {error}");
            return;
        }
    };

    if cli.verbose {
        for (name, value) in response.headers() {
            let value = value.to_str().unwrap_or("[non-UTF8]");
            println!(
                "← Header: {}: {}",
                name,
                display_header_value(name.as_str(), value)
            );
        }
    }

    let status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            eprintln!("Impossible de lire la réponse : {error}");
            return;
        }
    };
    let finish = Instant::now();
    let duration = finish - start;

    let duration_ms = duration.as_secs_f64() * 1000.0;

    println!("---");
    println!("→ {:?} {:?}", cli.method, cli.url);
    println!(
        "← {} · {:.2}ms · {}",
        status,
        duration_ms,
        format_size(body.len())
    );
    println!("---");

    if !body.is_empty() {
        println!("Body : {body}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_arguments() {
        let cli = Cli::parse_from(["rully", "GET", "https://example.com"]);
        assert_eq!(cli.method, Method::Get);
        assert_eq!(cli.url, "https://example.com");
    }

    #[test]
    fn missing_argument_method() {
        let result = Cli::try_parse_from(["rully", "https://example.com"]);
        assert!(result.is_err());
    }

    #[test]
    fn missing_argument_url() {
        let result = Cli::try_parse_from(["rully", "GET"]);
        assert!(result.is_err());
    }

    #[test]
    fn extra_arguments() {
        let result = Cli::try_parse_from(["rully", "GET", "https://example.com", ""]);
        assert!(result.is_err());
    }

    #[test]
    fn parse_all_methods() {
        let methods = [
            ("GET", Method::Get),
            ("POST", Method::Post),
            ("PUT", Method::Put),
            ("PATCH", Method::Patch),
            ("DELETE", Method::Delete),
        ];
        for (input, expected) in methods {
            let cli = Cli::try_parse_from(["rully", input, "https://example.com"])
                .expect("la méthode devrait être valide");
            assert_eq!(cli.method, expected);
        }
    }

    #[test]
    fn no_headers_by_default() {
        let cli = Cli::parse_from(["rully", "GET", "https://example.com"]);
        assert!(cli.headers.is_empty());
    }
    #[test]
    fn parse_one_header() {
        let cli = Cli::parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Authorization: Bearer test-token",
        ]);
        assert_eq!(cli.headers, vec!["Authorization: Bearer test-token"]);
    }
    #[test]
    fn parse_multiple_headers() {
        let cli = Cli::parse_from([
            "rully",
            "GET",
            "https://example.com",
            "--header",
            "Authorization: Bearer test-token",
            "--header",
            "Content-Type: application/json",
        ]);
        assert_eq!(
            cli.headers,
            vec![
                "Authorization: Bearer test-token",
                "Content-Type: application/json",
            ]
        );
    }

    #[test]
    fn verbose_is_disabled_by_default() {
        let cli = Cli::parse_from(["rully", "GET", "https://example.com"]);
        assert!(!cli.verbose);
    }

    #[test]
    fn parse_verbose_flag() {
        let cli = Cli::parse_from(["rully", "GET", "https://example.com", "--verbose"]);
        assert!(cli.verbose);
    }
}

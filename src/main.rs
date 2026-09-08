use clap::{Parser, ValueEnum};
use std::time::Instant;

#[derive(Clone, Debug, ValueEnum, PartialEq)]
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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!("méthode : {:?}", cli.method);
    println!("URL : {:?}", cli.url);
    println!("------");

    let start = Instant::now();

    let client = reqwest::Client::new();

    let method = match cli.method {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
        Method::Put => reqwest::Method::PUT,
        Method::Patch => reqwest::Method::PATCH,
        Method::Delete => reqwest::Method::DELETE,
    };

    let mut request = client.request(method, &cli.url);

    for header in cli.headers {
        let (name, value) = header
            .split_once(':')
            .expect("format attendu : Nom: valeur");
        request = request.header(name.trim(), value.trim());
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Erreur réseau : {error}");
            return;
        }
    };

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

    println!("Status : {status}");

    if !body.is_empty() {
        println!("Body : {body}");
    }

    println!("duration : {:?}", duration);
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
}

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    println!("méthode : {:?}", cli.method);
    println!("URL : {:?}", cli.url);
    println!("------");

    if cli.method == Method::Get {
        let start = Instant::now();

        let response = match reqwest::get(&cli.url).await {
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
    } else {
        println!("Méthode non supportée : {:?}", cli.method);
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
}

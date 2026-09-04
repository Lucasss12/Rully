use clap::{Parser, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
#[derive(PartialEq)]
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

fn main() {
    let cli = Cli::parse();
    
    println!("méthode : {:?}", cli.method);
    println!("URL : {:?}", cli.url);
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
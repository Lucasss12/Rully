mod cli;
mod errors;
mod http;
mod output;
mod request;
mod response;
mod style;
use errors::AppError;
use errors::ErrorCategory;
use request::Request;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        match error.category() {
            ErrorCategory::Cli => std::process::exit(2),
            _ => std::process::exit(1),
        }
    }
}

async fn run() -> Result<(), AppError> {
    let cli = cli::parse_args();
    let request = Request::try_from(&cli)?;
    let response = http::execute(&request).await?;
    output::display_response(&response, &request, cli.verbose);

    Ok(())
}

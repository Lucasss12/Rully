mod cli;
mod output;
mod style;
use rully_core::errors::AppError;
use rully_core::errors::ErrorCategory;
use rully_core::http;
use rully_core::request::Request;

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
    let client = http::default_client()?;
    let response = http::execute(&client, &request).await?;
    output::display_response(&response, &request, cli.verbose);

    Ok(())
}

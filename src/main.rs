use std::time::Instant;
mod cli;
mod errors;
mod http;
mod output;
mod style;
use errors::AppError;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError> {
    let start = Instant::now();

    let cli = cli::parse_args();
    let response = http::execute(&cli).await?;

    let status = response.status();
    let response_headers = response.headers().clone();
    let body = response.text().await.map_err(AppError::ResponseBody)?;

    let finish = Instant::now();
    let duration = finish - start;
    let duration_ms = duration.as_secs_f64() * 1000.0;

    output::display_response(&status, &response_headers, &body, &cli, duration_ms);

    Ok(())
}

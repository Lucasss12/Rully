use std::time::Instant;
mod cli;
mod errors;
mod http;
use errors::AppError;

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
    let body = response.text().await.map_err(AppError::ResponseBody)?;

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

    Ok(())
}

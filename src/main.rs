use std::time::Instant;
mod cli;

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
async fn main()  {
    let cli = cli::parse_args();

    let start = Instant::now();

    let client = reqwest::Client::new();

    let method = match cli.method {
        cli::Method::Get => reqwest::Method::GET,
        cli::Method::Post => reqwest::Method::POST,
        cli::Method::Put => reqwest::Method::PUT,
        cli::Method::Patch => reqwest::Method::PATCH,
        cli::Method::Delete => reqwest::Method::DELETE,
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
    
    let body = match (cli.body.as_deref(), cli.body_file.as_deref()) {
        (Some(_), Some(_)) => {
            eprintln!("--body et --body-file ne peuvent pas être utilisés ensemble");
            return;
        }
        
        (Some(body), None) => Some(body.to_owned()),
        (None, Some(path)) => match std::fs::read_to_string(path) {
            Ok(content) => Some(content),
            Err(error) => {
                eprintln!("Impossible de lire le fichier : {error}");
                return;
            }
        },
        (None, None) => None,
    };
    
    let is_json = cli.headers.iter().any(|header| {
        let Some((name, value)) = header.split_once(':') else {
            return false;
        };
        let media_type = value
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        name.trim().eq_ignore_ascii_case("content-type")
            && media_type.eq_ignore_ascii_case("application/json")
    });

    if let Some(body) = body.as_deref() {
        if is_json {
            match serde_json::from_str::<serde_json::Value>(body) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("JSON invalide : {error}");
                    return;
                }
            }
        }
        request = request.body(body.to_owned());
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

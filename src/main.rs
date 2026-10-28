mod cli;
mod output;
mod style;
mod tui;
use rully_core::errors::AppError;
use rully_core::errors::ErrorCategory;
use rully_core::http;
use rully_core::http_file;
use rully_core::request::Request;
use rully_core::store;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

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
    cli.validate()?;

    let cwd = std::env::current_dir()
        .map_err(|error| AppError::FileRead(std::io::Error::other(error.to_string())))?;

    match &cli.command {
        Some(cli::Command::Init) => {
            let root = cwd.join(store::ROOT_DIR);
            store::init(&root)?;

            println!("Created {}", style::cyan(&root.display().to_string()));
        }
        Some(cli::Command::Tui) => {
            let root = open_or_init_workspace(&cwd)?;
            tui::run(root)?;
        }

        Some(cli::Command::New { name }) => {
            let root = store::open(&cwd)?;

            if http_file::is_request_path(name) {
                let path = store::new_request(&root, name)?;

                println!(
                    "Created request {}",
                    style::cyan(&path.display().to_string())
                );
            } else {
                store::create_collection(&root, name)?;

                println!("Created collection {}", style::cyan(name));
            }
        }
        Some(cli::Command::List { collection }) => {
            let root = store::open(&cwd)?;

            match collection {
                Some(name) => {
                    let requests = store::list_requests(&root, name)?;
                    output::display_requests(&requests);
                }
                None => {
                    let collections = store::list_collections(&root)?;
                    output::display_collections(&collections);
                }
            }
        }
        Some(cli::Command::Run { path }) => {
            let request = load_request(&cwd, path)?;
            execute(&request, cli.verbose).await?;
        }
        Some(cli::Command::Edit { path }) => edit(&cwd, path)?,
        Some(cli::Command::Delete { path }) => {
            let root = store::open(&cwd)?;
            store::delete(&root, path)?;

            println!("Deleted {}", style::cyan(path));
        }
        None => {
            let request = Request::try_from(&cli)?;

            if let Some(name) = &cli.save {
                save(&cwd, name, &request)?;
            }

            execute(&request, cli.verbose).await?;
        }
    }

    Ok(())
}

/// Opens the workspace for the TUI, creating it on first use.
///
/// The TUI is a first-use entry point, so a missing workspace is initialized
/// instead of reported. A workspace that exists but is invalid or only partly
/// created is an error: the TUI never repairs it silently.
fn open_or_init_workspace(cwd: &Path) -> Result<PathBuf, AppError> {
    match store::open(cwd) {
        Ok(root) => Ok(root),
        Err(AppError::WorkspaceNotFound) => {
            let root = cwd.join(store::ROOT_DIR);
            store::init(&root)?;
            Ok(root)
        }
        Err(error) => Err(error),
    }
}

/// Saves the request before sending it, so the capture works offline and a
/// failing request is exactly the one you want to keep.
fn save(cwd: &Path, name: &str, request: &Request) -> Result<(), AppError> {
    let root = store::open(cwd)?;
    let content = http_file::to_string(request)?;
    let path = store::save(&root, name, &content)?;

    output::display_saved(&path.display().to_string());

    Ok(())
}

/// Hands the file to the user's editor, then re-reads it so a bad edit is
/// reported straight away. Rully never rewrites the file itself, which is what
/// keeps comments and formatting intact.
fn edit(cwd: &Path, path: &str) -> Result<(), AppError> {
    let root = store::open(cwd)?;
    let full_path = store::request_path(&root, path)?;

    if !full_path.is_file() {
        return Err(AppError::RequestNotFound(path.to_string()));
    }

    let editor = std::env::var("VISUAL")
        .ok()
        .or_else(|| std::env::var("EDITOR").ok())
        .ok_or(AppError::NoEditor)?;

    let before = std::fs::read_to_string(&full_path).map_err(AppError::FileRead)?;

    let status = Process::new(&editor)
        .arg(&full_path)
        .status()
        .map_err(|error| AppError::EditorFailed(error.to_string()))?;

    if !status.success() {
        return Err(AppError::EditorFailed(format!(
            "{editor} exited with {status}"
        )));
    }

    let after = std::fs::read_to_string(&full_path).map_err(AppError::FileRead)?;

    if before == after {
        output::display_unchanged(path);
    } else {
        http_file::read(&full_path.to_string_lossy()).and_then(Request::try_from)?;

        output::display_edited(path);
    }

    Ok(())
}

/// Resolves a request path inside the workspace and turns it into a `Request`.
fn load_request(cwd: &Path, path: &str) -> Result<Request, AppError> {
    let root = store::open(cwd)?;
    let full_path = store::request_path(&root, path)?;

    if !full_path.is_file() {
        return Err(AppError::RequestNotFound(path.to_string()));
    }

    let file = http_file::read(&full_path.to_string_lossy())?;

    Request::try_from(file)
}

async fn execute(request: &Request, verbose: bool) -> Result<(), AppError> {
    let client = http::default_client()?;
    let response = http::execute(&client, request).await?;
    output::display_response(&response, request, verbose);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rully_core::store;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("rully-main-{name}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();

            Self { path }
        }

        fn join(&self, segment: &str) -> PathBuf {
            self.path.join(segment)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn initializes_a_workspace_for_the_tui() {
        let cwd = TempDir::new("tui-init");

        let root = open_or_init_workspace(&cwd.path).unwrap();

        assert_eq!(root, cwd.join(store::ROOT_DIR));
        assert!(store::collections_dir(&root).is_dir());
        assert_eq!(store::read_format(&root).unwrap(), store::SUPPORTED_FORMAT);
    }

    #[test]
    fn opens_an_existing_workspace() {
        let cwd = TempDir::new("tui-open");
        let expected = open_or_init_workspace(&cwd.path).unwrap();

        let root = open_or_init_workspace(&cwd.path).unwrap();

        assert_eq!(root, expected);
    }

    #[test]
    fn opens_a_workspace_from_a_subdirectory() {
        let cwd = TempDir::new("tui-subdirectory");
        let expected = open_or_init_workspace(&cwd.path).unwrap();
        let nested = cwd.join("src/tools");
        std::fs::create_dir_all(&nested).unwrap();

        let root = open_or_init_workspace(&nested).unwrap();

        assert_eq!(root, expected);
    }

    #[test]
    fn never_reinitializes_an_existing_workspace() {
        let cwd = TempDir::new("tui-no-reinit");
        let first = open_or_init_workspace(&cwd.path).unwrap();
        let config = first.join(store::CONFIG_FILE);
        let before = std::fs::read_to_string(&config).unwrap();

        let second = open_or_init_workspace(&cwd.path).unwrap();

        assert_eq!(second, first);
        assert_eq!(std::fs::read_to_string(config).unwrap(), before);
    }

    #[test]
    fn reports_an_invalid_workspace_instead_of_initializing() {
        let cwd = TempDir::new("tui-invalid");
        let root = cwd.join(store::ROOT_DIR);
        std::fs::create_dir_all(store::collections_dir(&root)).unwrap();
        std::fs::write(root.join(store::CONFIG_FILE), "{\"format\": 999}").unwrap();

        let result = open_or_init_workspace(&cwd.path);

        assert!(matches!(
            result,
            Err(AppError::HttpVersionUnsupported { .. })
        ));
    }
}

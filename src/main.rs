mod tui;

use rully_core::errors::AppError;
use rully_core::store;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    // The binary launches the TUI, always. Flags are binary metadata only:
    // anything else is ignored rather than rejected, keeping the interface
    // down to a single command.
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-V" | "--version" => {
                println!("rully {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => {}
        }
    }

    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError> {
    let cwd = std::env::current_dir()
        .map_err(|error| AppError::FileRead(std::io::Error::other(error.to_string())))?;

    let root = open_or_init_workspace(&cwd)?;
    tui::run(root)?;

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

fn print_help() {
    println!("Usage: rully [option]");
    println!();
    println!("Launches the interactive terminal client over the collections");
    println!("stored in `./rully/collections`.");
    println!();
    println!("Options:");
    println!("  -h, --help       Print this help");
    println!("  -V, --version    Print version");
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

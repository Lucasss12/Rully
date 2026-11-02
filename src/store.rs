use crate::errors::AppError;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const ROOT_DIR: &str = "rully";
pub const COLLECTIONS_DIR: &str = "collections";
pub const ENVIRONMENTS_DIR: &str = "environments";
pub const CONFIG_FILE: &str = "rully.json";
pub const SUPPORTED_FORMAT: u32 = 1;

/// Lines appended to the workspace `.gitignore` so real environment files
/// stay local while `example.env` remains shareable.
const GITIGNORE_ENV_LINES: [&str; 2] = ["environments/*.env", "!environments/example.env"];

const EXAMPLE_ENV: &str = "# Copy this file to <name>.env and fill in your values.\n\
                           # Real .env files are gitignored.\n\
                           base_url = https://api.example.com\n";

/// The skeleton written when creating an environment from the TUI.
const ENV_TEMPLATE: &str = "# One variable per line: KEY = value\n\
                            # Lines starting with # are comments.\n";

pub fn find_root(start: &Path) -> Result<PathBuf, AppError> {
    let mut current = Some(start);

    while let Some(directory) = current {
        let candidate = directory.join(ROOT_DIR);

        if candidate.join(CONFIG_FILE).is_file() {
            return Ok(candidate);
        }

        current = directory.parent();
    }

    Err(AppError::WorkspaceNotFound)
}

/// Locates the workspace from `cwd` and checks that its format is supported.
///
/// Every command that touches the workspace goes through this, so a version
/// check can never be forgotten on one code path.
pub fn open(start: &Path) -> Result<PathBuf, AppError> {
    let root = find_root(start)?;
    read_format(&root)?;

    Ok(root)
}

pub fn collections_dir(root: &Path) -> PathBuf {
    root.join(COLLECTIONS_DIR)
}

pub fn environments_dir(root: &Path) -> PathBuf {
    root.join(ENVIRONMENTS_DIR)
}

pub fn read_format(root: &Path) -> Result<u32, AppError> {
    let path = root.join(CONFIG_FILE);
    let source = fs::read_to_string(&path).map_err(AppError::FileRead)?;
    let value: serde_json::Value = serde_json::from_str(&source).map_err(AppError::InvalidJson)?;

    let found = value
        .get("format")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            AppError::InvalidHttpFile("`format` must be a number in rully.json".to_string())
        })? as u32;

    if found > SUPPORTED_FORMAT {
        return Err(AppError::HttpVersionUnsupported {
            found,
            supported: SUPPORTED_FORMAT,
        });
    }

    Ok(found)
}

pub fn init(root: &Path) -> Result<(), AppError> {
    let collections = collections_dir(root);

    if collections.exists() {
        return Err(AppError::WorkspaceAlreadyExists);
    }

    fs::create_dir_all(&collections).map_err(AppError::FileWrite)?;
    write_config(root)?;
    write_example_environment(root)?;
    write_env_gitignore(root)?;

    Ok(())
}

fn write_config(root: &Path) -> Result<(), AppError> {
    let path = root.join(CONFIG_FILE);
    let config = format!("{{\n  \"format\": {SUPPORTED_FORMAT}\n}}\n");

    fs::write(&path, config).map_err(AppError::FileWrite)
}

/// Seeds `environments/example.env` so a fresh workspace teaches the format
/// without ever containing a real secret.
fn write_example_environment(root: &Path) -> Result<(), AppError> {
    let directory = environments_dir(root);
    fs::create_dir_all(&directory).map_err(AppError::FileWrite)?;

    fs::write(directory.join("example.env"), EXAMPLE_ENV).map_err(AppError::FileWrite)
}

/// Keeps real `.env` files out of Git. An existing `.gitignore` is extended,
/// never replaced.
fn write_env_gitignore(root: &Path) -> Result<(), AppError> {
    let path = root.join(".gitignore");
    let mut content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(AppError::FileRead(error)),
    };

    for line in GITIGNORE_ENV_LINES {
        if !content.lines().any(|existing| existing.trim() == line) {
            content.push_str(line);
            content.push('\n');
        }
    }

    fs::write(&path, content).map_err(AppError::FileWrite)
}

pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let mut path = collections_dir(root);

    for segment in relative.split('/') {
        validate_name(segment)?;
        path.push(segment);
    }

    Ok(path)
}

/// Creates a collection, including any missing parent directory. A nested
/// name such as `users/admin` creates `users/` too.
pub fn create_collection(root: &Path, collection: &str) -> Result<(), AppError> {
    let path = resolve(root, collection)?;

    if path.exists() {
        return Err(AppError::CollectionAlreadyExists(collection.to_string()));
    }

    fs::create_dir_all(&path).map_err(AppError::FileWrite)
}

pub fn rename_collection(root: &Path, from: &str, to: &str) -> Result<PathBuf, AppError> {
    let source = resolve(root, from)?;
    let target = resolve(root, to)?;

    if !source.is_dir() {
        return Err(AppError::CollectionNotFound(from.to_string()));
    }

    if from == to {
        return Ok(source);
    }

    if target.exists() {
        return Err(AppError::CollectionAlreadyExists(to.to_string()));
    }

    fs::rename(&source, &target).map_err(AppError::FileWrite)?;

    Ok(target)
}

pub fn delete_collection(root: &Path, collection: &str) -> Result<(), AppError> {
    let path = resolve(root, collection)?;

    if !path.is_dir() {
        return Err(AppError::CollectionNotFound(collection.to_string()));
    }

    let mut entries = fs::read_dir(&path).map_err(AppError::FileRead)?;

    match entries.next() {
        None => {}
        Some(Ok(_)) => {
            return Err(AppError::CollectionNotEmpty(collection.to_string()));
        }
        Some(Err(error)) => return Err(AppError::FileRead(error)),
    }

    fs::remove_dir(&path).map_err(AppError::FileWrite)
}

/// Writes a request, refusing to touch an existing file.
///
/// The write goes to a temporary file that is then renamed over the target, so
/// an interrupted save can never leave a half-written request behind.
pub fn save(root: &Path, relative: &str, content: &str) -> Result<PathBuf, AppError> {
    let path = request_path(root, relative)?;

    if path.exists() {
        return Err(AppError::RequestAlreadyExists(relative.to_string()));
    }

    let collection = path
        .parent()
        .ok_or_else(|| AppError::RequestNotFound(relative.to_string()))?;

    if !collection.is_dir() {
        return Err(missing_collection(collection));
    }

    write_atomic(&path, content)?;

    Ok(path)
}

/// Rewrites a request that already exists.
///
/// The TUI can only save a request it loaded from disk, so a missing file is an
/// error rather than a request to create one implicitly.
pub fn overwrite(root: &Path, relative: &str, content: &str) -> Result<PathBuf, AppError> {
    let path = request_path(root, relative)?;

    if !path.is_file() {
        return Err(AppError::RequestNotFound(relative.to_string()));
    }

    write_atomic(&path, content)?;

    Ok(path)
}

fn write_atomic(path: &Path, content: &str) -> Result<(), AppError> {
    let temporary = path.with_extension("http.tmp");
    fs::write(&temporary, content).map_err(AppError::FileWrite)?;

    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(AppError::FileWrite(error));
    }

    Ok(())
}

/// Reports a missing collection together with the command that creates it.
fn missing_collection(collection: &Path) -> AppError {
    let name = collection
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    AppError::MissingCollection(name)
}

pub fn delete(root: &Path, relative: &str) -> Result<(), AppError> {
    let path = request_path(root, relative)?;

    if !path.is_file() {
        return Err(AppError::RequestNotFound(relative.to_string()));
    }

    fs::remove_file(&path).map_err(AppError::FileWrite)
}

pub fn rename_request(root: &Path, from: &str, to: &str) -> Result<PathBuf, AppError> {
    let source = request_path(root, from)?;
    let target = request_path(root, to)?;

    if !source.is_file() {
        return Err(AppError::RequestNotFound(from.to_string()));
    }

    if source == target {
        return Ok(source);
    }

    let collection = target
        .parent()
        .ok_or_else(|| AppError::RequestNotFound(to.to_string()))?;

    if !collection.is_dir() {
        return Err(missing_collection(collection));
    }

    if target.exists() {
        return Err(AppError::RequestAlreadyExists(to.to_string()));
    }

    fs::rename(&source, &target).map_err(AppError::FileWrite)?;

    Ok(target)
}

/// Creates a request from the template. The collection must already exist, so
/// the workspace layout is never built implicitly.
pub fn new_request(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    save(root, relative, crate::http_file::template())
}

/// Resolves a request path, appending the `.http` extension when missing so
/// both `users/list` and `users/list.http` name the same file.
pub fn request_path(root: &Path, relative: &str) -> Result<PathBuf, AppError> {
    let relative = if relative.ends_with(".http") {
        relative.to_string()
    } else {
        format!("{relative}.http")
    };

    resolve(root, &relative)
}

pub fn validate_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() || name == "." || name == ".." {
        return Err(AppError::InvalidRequestName(name.to_string()));
    }

    // `/` is absent on purpose: `resolve` splits the relative path on `/`
    // before validating each segment, so a segment can never contain one.
    let forbidden = ['<', '>', ':', '"', '\\', '|', '?', '*'];

    if name.contains(forbidden) {
        return Err(AppError::InvalidRequestName(name.to_string()));
    }

    if name.trim() != name {
        return Err(AppError::InvalidRequestName(name.to_string()));
    }

    Ok(())
}

pub fn list_collections(root: &Path) -> Result<Vec<String>, AppError> {
    list_dir(&collections_dir(root))
}

/// Lists the environment names (file stems of `*.env`), sorted. A workspace
/// created before environments existed has no directory yet, which lists as
/// empty rather than an error.
pub fn list_environments(root: &Path) -> Result<Vec<String>, AppError> {
    let directory = environments_dir(root);

    if !directory.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&directory).map_err(AppError::FileRead)?;
    let mut names = Vec::new();

    for entry in entries {
        let entry = entry.map_err(AppError::FileRead)?;
        let path = entry.path();

        if path.extension().is_some_and(|extension| extension == "env")
            && let Some(name) = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        {
            names.push(name);
        }
    }

    names.sort();

    Ok(names)
}

/// Resolves an environment name to its file, rejecting anything that could
/// escape the environments directory. Both `dev` and `dev.env` name the same
/// file, matching how the TUI lists file stems.
pub fn environment_path(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    if name.contains('/') {
        return Err(AppError::InvalidRequestName(name.to_string()));
    }
    validate_name(name)?;

    let file = if name.ends_with(".env") {
        name.to_string()
    } else {
        format!("{name}.env")
    };

    Ok(environments_dir(root).join(file))
}

/// Reads one environment file into its variables.
pub fn read_environment(root: &Path, name: &str) -> Result<HashMap<String, String>, AppError> {
    let path = environment_path(root, name)?;

    if !path.is_file() {
        return Err(AppError::EnvironmentNotFound(name.to_string()));
    }

    crate::env_file::read(&path)
}

/// Reads one environment file as written, placeholders, comments and all —
/// what the TUI editor loads so a save never reformats the file.
pub fn read_environment_text(root: &Path, name: &str) -> Result<String, AppError> {
    let path = environment_path(root, name)?;

    if !path.is_file() {
        return Err(AppError::EnvironmentNotFound(name.to_string()));
    }

    fs::read_to_string(&path).map_err(AppError::FileRead)
}

/// Creates an environment from the template, refusing to touch an existing
/// file.
pub fn create_environment(root: &Path, name: &str) -> Result<PathBuf, AppError> {
    let path = environment_path(root, name)?;

    if path.exists() {
        return Err(AppError::EnvironmentAlreadyExists(name.to_string()));
    }

    fs::create_dir_all(environments_dir(root)).map_err(AppError::FileWrite)?;
    write_atomic(&path, ENV_TEMPLATE)?;

    Ok(path)
}

/// Renames an environment file, refusing to overwrite another one.
pub fn rename_environment(root: &Path, from: &str, to: &str) -> Result<PathBuf, AppError> {
    let source = environment_path(root, from)?;
    let target = environment_path(root, to)?;

    if !source.is_file() {
        return Err(AppError::EnvironmentNotFound(from.to_string()));
    }

    if source == target {
        return Ok(source);
    }

    if target.exists() {
        return Err(AppError::EnvironmentAlreadyExists(to.to_string()));
    }

    fs::rename(&source, &target).map_err(AppError::FileWrite)?;

    Ok(target)
}

pub fn delete_environment(root: &Path, name: &str) -> Result<(), AppError> {
    let path = environment_path(root, name)?;

    if !path.is_file() {
        return Err(AppError::EnvironmentNotFound(name.to_string()));
    }

    fs::remove_file(&path).map_err(AppError::FileWrite)
}

/// Rewrites an environment that already exists. The content is written
/// atomically; callers validate it first.
pub fn overwrite_environment(root: &Path, name: &str, content: &str) -> Result<PathBuf, AppError> {
    let path = environment_path(root, name)?;

    if !path.is_file() {
        return Err(AppError::EnvironmentNotFound(name.to_string()));
    }

    write_atomic(&path, content)?;

    Ok(path)
}

/// The environment selected in `rully.json`, if any.
pub fn active_environment(root: &Path) -> Result<Option<String>, AppError> {
    let source = fs::read_to_string(root.join(CONFIG_FILE)).map_err(AppError::FileRead)?;
    let value: serde_json::Value = serde_json::from_str(&source).map_err(AppError::InvalidJson)?;

    Ok(value
        .get("environment")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string))
}

/// Stores the selected environment in `rully.json`, preserving the other keys.
pub fn set_active_environment(root: &Path, name: &str) -> Result<(), AppError> {
    let path = root.join(CONFIG_FILE);
    let source = fs::read_to_string(&path).map_err(AppError::FileRead)?;
    let mut value: serde_json::Value =
        serde_json::from_str(&source).map_err(AppError::InvalidJson)?;

    let object = value.as_object_mut().ok_or_else(|| {
        AppError::InvalidHttpFile("`rully.json` must contain a JSON object".to_string())
    })?;
    object.insert(
        "environment".to_string(),
        serde_json::Value::String(name.to_string()),
    );

    let formatted = serde_json::to_string_pretty(&value).map_err(AppError::InvalidJson)?;

    fs::write(&path, format!("{formatted}\n")).map_err(AppError::FileWrite)
}

/// Removes the selected environment from `rully.json`, keeping the other
/// keys — used when the active environment is deleted.
pub fn clear_active_environment(root: &Path) -> Result<(), AppError> {
    let path = root.join(CONFIG_FILE);
    let source = fs::read_to_string(&path).map_err(AppError::FileRead)?;
    let mut value: serde_json::Value =
        serde_json::from_str(&source).map_err(AppError::InvalidJson)?;

    let object = value.as_object_mut().ok_or_else(|| {
        AppError::InvalidHttpFile("`rully.json` must contain a JSON object".to_string())
    })?;
    object.remove("environment");

    let formatted = serde_json::to_string_pretty(&value).map_err(AppError::InvalidJson)?;

    fs::write(&path, format!("{formatted}\n")).map_err(AppError::FileWrite)
}

/// Lists the direct contents of a collection: its requests, and the
/// collections nested inside it.
pub fn list_requests(root: &Path, collection: &str) -> Result<Vec<String>, AppError> {
    let path = resolve(root, collection)?;

    if !path.is_dir() {
        return Err(AppError::CollectionNotFound(collection.to_string()));
    }

    list_dir(&path)
}

fn list_dir(path: &Path) -> Result<Vec<String>, AppError> {
    let entries = fs::read_dir(path).map_err(AppError::FileRead)?;
    let mut names = Vec::new();

    for entry in entries {
        let entry = entry.map_err(AppError::FileRead)?;
        let path = entry.path();

        let Some(name) = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        else {
            continue;
        };

        if path.is_dir() {
            names.push(format!("{name}/"));
        } else if path.extension().is_some_and(|ext| ext == "http") {
            names.push(name);
        }
    }

    names.sort();

    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Request;

    fn workspace(name: &str) -> PathBuf {
        let project = std::env::temp_dir().join(format!("rully-store-{name}"));
        let _ = fs::remove_dir_all(&project);
        fs::create_dir_all(&project).unwrap();
        project.join(ROOT_DIR)
    }

    /// Every fixture leaves a directory behind. Drop this to clean it.
    struct Project(PathBuf);

    impl Project {
        fn new(root: &Path) -> Self {
            Self(root.parent().unwrap().to_path_buf())
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn workspace_with_content(name: &str) -> PathBuf {
        let root = workspace(name);
        init(&root).unwrap();
        let users = collections_dir(&root).join("users");
        fs::create_dir_all(&users).unwrap();
        fs::write(users.join("list.http"), "GET https://api.example.com").unwrap();
        fs::write(users.join("create.http"), "POST https://api.example.com").unwrap();
        fs::create_dir_all(collections_dir(&root).join("auth")).unwrap();
        fs::write(
            collections_dir(&root).join("auth").join("login.http"),
            "POST https://api.example.com/login",
        )
        .unwrap();
        root
    }

    #[test]
    fn finds_the_root_in_the_current_directory() {
        let root = workspace_with_content("root-cwd");
        let _project = Project::new(&root);

        assert_eq!(find_root(&root).unwrap(), root);
    }

    #[test]
    fn finds_the_root_from_a_subdirectory() {
        let root = workspace_with_content("root-sub");
        let _project = Project::new(&root);
        let nested = collections_dir(&root).join("users");

        assert_eq!(find_root(&nested).unwrap(), root);
    }

    #[test]
    fn reports_a_missing_workspace() {
        let result = find_root(&std::env::temp_dir());

        assert!(matches!(result, Err(AppError::WorkspaceNotFound)));
    }

    #[test]
    fn init_creates_the_collections_directory() {
        let root = workspace("init-collections");
        let _project = Project::new(&root);

        init(&root).unwrap();

        assert!(collections_dir(&root).is_dir());
    }

    #[test]
    fn init_writes_the_config() {
        let root = workspace("init-config");
        let _project = Project::new(&root);

        init(&root).unwrap();

        assert_eq!(read_format(&root).unwrap(), SUPPORTED_FORMAT);
    }

    #[test]
    fn refuses_to_init_twice() {
        let root = workspace("init-twice");
        let _project = Project::new(&root);

        init(&root).unwrap();
        let result = init(&root);

        assert!(matches!(result, Err(AppError::WorkspaceAlreadyExists)));
    }

    #[test]
    fn rejects_an_unsupported_format() {
        let root = workspace_with_content("format-unsupported");
        let _project = Project::new(&root);
        fs::write(root.join(CONFIG_FILE), "{\n  \"format\": 99\n}").unwrap();

        let result = read_format(&root);

        assert!(matches!(
            result,
            Err(AppError::HttpVersionUnsupported {
                found: 99,
                supported: SUPPORTED_FORMAT,
            })
        ));
    }

    #[test]
    fn open_rejects_an_unsupported_format() {
        let root = workspace_with_content("format-open");
        let _project = Project::new(&root);
        fs::write(root.join(CONFIG_FILE), "{\n  \"format\": 99\n}").unwrap();

        let result = open(&root);

        assert!(matches!(
            result,
            Err(AppError::HttpVersionUnsupported { found: 99, .. })
        ));
    }

    #[test]
    fn open_returns_the_root_of_a_supported_workspace() {
        let root = workspace_with_content("format-open-ok");
        let _project = Project::new(&root);
        let nested = collections_dir(&root).join("users");

        assert_eq!(open(&nested).unwrap(), root);
    }

    #[test]
    fn open_reports_a_missing_workspace() {
        let root = workspace("open-missing");
        let _project = Project::new(&root);

        let result = open(&root);

        assert!(matches!(result, Err(AppError::WorkspaceNotFound)));
    }

    #[test]
    fn reports_a_missing_config() {
        let root = workspace("format-missing");
        let _project = Project::new(&root);

        let result = read_format(&root);

        assert!(matches!(result, Err(AppError::FileRead(_))));
    }

    #[test]
    fn resolves_a_path_inside_the_collections_directory() {
        let root = workspace_with_content("resolve");
        let _project = Project::new(&root);

        let path = resolve(&root, "users/list.http").unwrap();

        assert_eq!(path, collections_dir(&root).join("users").join("list.http"));
    }

    #[test]
    fn resolves_a_nested_path() {
        let root = workspace_with_content("resolve-nested");
        let _project = Project::new(&root);

        let path = resolve(&root, "users/admin/delete.http").unwrap();

        assert!(path.ends_with("users/admin/delete.http"));
    }

    #[test]
    fn rejects_a_path_that_escapes_the_workspace() {
        let root = workspace_with_content("resolve-escape");
        let _project = Project::new(&root);

        let result = resolve(&root, "../../etc/passwd");

        assert!(matches!(result, Err(AppError::InvalidRequestName(_))));
    }

    #[test]
    fn accepts_a_regular_name() {
        assert!(validate_name("list-users").is_ok());
        assert!(validate_name("list.users").is_ok());
    }

    #[test]
    fn rejects_names_forbidden_on_windows() {
        for name in ["a<b", "a>b", "a:b", "a\"b", "a|b", "a?b", "a*b"] {
            assert!(
                matches!(validate_name(name), Err(AppError::InvalidRequestName(_))),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_an_empty_or_relative_name() {
        for name in ["", ".", ".."] {
            assert!(
                matches!(validate_name(name), Err(AppError::InvalidRequestName(_))),
                "{name} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_a_name_with_surrounding_spaces() {
        assert!(matches!(
            validate_name(" name "),
            Err(AppError::InvalidRequestName(_))
        ));
    }

    #[test]
    fn lists_the_collections() {
        let root = workspace_with_content("list-collections");
        let _project = Project::new(&root);

        let names = list_collections(&root).unwrap();

        assert_eq!(names, vec!["auth/", "users/"]);
    }

    #[test]
    fn lists_the_requests_of_a_collection() {
        let root = workspace_with_content("list-requests");
        let _project = Project::new(&root);

        let names = list_requests(&root, "users").unwrap();

        assert_eq!(names, vec!["create.http", "list.http"]);
    }

    #[test]
    fn ignores_files_that_are_not_requests() {
        let root = workspace_with_content("list-ignore");
        let _project = Project::new(&root);
        let users = collections_dir(&root).join("users");
        fs::write(users.join("notes.txt"), "hello").unwrap();

        let names = list_requests(&root, "users").unwrap();

        assert_eq!(names, vec!["create.http", "list.http"]);
    }

    #[test]
    fn creates_a_collection() {
        let root = workspace("new-collection");
        let _project = Project::new(&root);
        init(&root).unwrap();

        create_collection(&root, "users").unwrap();

        assert!(collections_dir(&root).join("users").is_dir());
    }

    #[test]
    fn refuses_to_create_a_collection_twice() {
        let root = workspace("new-collection-twice");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let result = create_collection(&root, "users");

        assert!(matches!(result, Err(AppError::CollectionAlreadyExists(_))));
    }

    #[test]
    fn refuses_a_collection_name_with_forbidden_characters() {
        let root = workspace("new-collection-invalid");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = create_collection(&root, "bad<name>");

        assert!(matches!(result, Err(AppError::InvalidRequestName(_))));
    }

    #[test]
    fn creates_a_nested_collection() {
        let root = workspace("new-collection-nested");
        let _project = Project::new(&root);
        init(&root).unwrap();

        create_collection(&root, "users/admin").unwrap();

        assert!(collections_dir(&root).join("users/admin").is_dir());
    }

    #[test]
    fn creating_a_nested_collection_creates_the_parent() {
        let root = workspace("new-collection-parent");
        let _project = Project::new(&root);
        init(&root).unwrap();

        create_collection(&root, "users/admin").unwrap();

        assert!(collections_dir(&root).join("users").is_dir());
    }

    #[test]
    fn refuses_to_create_a_nested_collection_twice() {
        let root = workspace("new-collection-nested-twice");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users/admin").unwrap();

        let result = create_collection(&root, "users/admin");

        assert!(matches!(result, Err(AppError::CollectionAlreadyExists(_))));
    }

    #[test]
    fn refuses_a_collection_name_that_escapes_the_workspace() {
        let root = workspace("new-collection-escape");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = create_collection(&root, "../elsewhere");

        assert!(matches!(result, Err(AppError::InvalidRequestName(_))));
    }

    #[test]
    fn renames_a_collection_with_its_contents() {
        let root = workspace("rename-collection");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        let path = rename_collection(&root, "users", "accounts").unwrap();

        assert!(path.is_dir());
        assert!(!collections_dir(&root).join("users").exists());
        assert!(collections_dir(&root).join("accounts/list.http").is_file());
    }

    #[test]
    fn renaming_a_collection_to_its_own_name_is_a_no_op() {
        let root = workspace("rename-collection-same");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let path = rename_collection(&root, "users", "users").unwrap();

        assert!(path.is_dir());
    }

    #[test]
    fn refuses_to_rename_a_missing_collection() {
        let root = workspace("rename-collection-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = rename_collection(&root, "users", "accounts");

        assert!(matches!(result, Err(AppError::CollectionNotFound(_))));
    }

    #[test]
    fn refuses_to_rename_onto_an_existing_collection() {
        let root = workspace("rename-collection-existing");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        create_collection(&root, "accounts").unwrap();

        let result = rename_collection(&root, "users", "accounts");

        assert!(matches!(result, Err(AppError::CollectionAlreadyExists(_))));
    }

    #[test]
    fn deletes_an_empty_collection() {
        let root = workspace("delete-collection-empty");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        delete_collection(&root, "users").unwrap();

        assert!(!collections_dir(&root).join("users").exists());
    }

    #[test]
    fn refuses_to_delete_a_non_empty_collection() {
        let root = workspace_with_content("delete-collection-non-empty");
        let _project = Project::new(&root);

        let result = delete_collection(&root, "users");

        assert!(matches!(result, Err(AppError::CollectionNotEmpty(_))));
        assert!(collections_dir(&root).join("users").is_dir());
    }

    #[test]
    fn refuses_to_delete_a_missing_collection() {
        let root = workspace("delete-collection-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = delete_collection(&root, "users");

        assert!(matches!(result, Err(AppError::CollectionNotFound(_))));
    }

    #[test]
    fn lists_a_nested_collection() {
        let root = workspace_with_content("list-nested");
        let _project = Project::new(&root);
        fs::create_dir_all(collections_dir(&root).join("users/admin")).unwrap();

        let names = list_requests(&root, "users/admin").unwrap();

        assert!(names.is_empty(), "{names:?}");
        assert!(
            list_requests(&root, "users")
                .unwrap()
                .contains(&"admin/".to_string())
        );
    }

    #[test]
    fn reports_a_missing_collection() {
        let root = workspace_with_content("list-missing");
        let _project = Project::new(&root);

        let result = list_requests(&root, "nope");

        assert!(matches!(result, Err(AppError::CollectionNotFound(_))));
    }

    #[test]
    fn reports_a_missing_collections_directory() {
        let root = workspace("list-no-dir");
        let _project = Project::new(&root);

        let result = list_collections(&root);

        assert!(matches!(result, Err(AppError::FileRead(_))));
    }

    #[test]
    fn reads_a_real_workspace_layout() {
        let root = workspace("layout-project");
        let _project = Project::new(&root);
        let users = root.join(COLLECTIONS_DIR).join("users");
        fs::create_dir_all(&users).unwrap();
        write_config(&root).unwrap();
        fs::write(
            users.join("list.http"),
            "GET https://api.example.com/users\nAccept: application/json",
        )
        .unwrap();

        let found = find_root(&users).unwrap();

        assert_eq!(found, root);
        assert_eq!(read_format(&found).unwrap(), SUPPORTED_FORMAT);
        assert_eq!(
            resolve(&found, "users/list.http").unwrap(),
            users.join("list.http")
        );
        assert_eq!(list_collections(&found).unwrap(), vec!["users/"]);
        assert_eq!(list_requests(&found, "users").unwrap(), vec!["list.http"]);

        let request = crate::http_file::read(
            resolve(&found, "users/list.http")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .and_then(Request::try_from)
        .unwrap();

        assert_eq!(request.headers["accept"], "application/json");
    }

    // save

    #[test]
    fn saves_a_request() {
        let root = workspace("save");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let path = save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "GET https://api.example.com"
        );
    }

    #[test]
    fn saves_without_the_extension() {
        let root = workspace("save-no-ext");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let path = save(&root, "users/list", "GET https://api.example.com").unwrap();

        assert_eq!(path.extension().unwrap(), "http");
    }

    #[test]
    fn refuses_to_overwrite_a_request() {
        let root = workspace("save-overwrite");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        let result = save(&root, "users/list.http", "GET https://other.example.com");

        assert!(matches!(result, Err(AppError::RequestAlreadyExists(_))));
        let kept = fs::read_to_string(collections_dir(&root).join("users/list.http")).unwrap();
        assert_eq!(kept, "GET https://api.example.com");
    }

    #[test]
    fn overwrites_an_existing_request() {
        let root = workspace("overwrite");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        let path = overwrite(&root, "users/list.http", "POST https://api.example.com").unwrap();

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "POST https://api.example.com"
        );
    }

    #[test]
    fn refuses_to_overwrite_a_missing_request() {
        let root = workspace("overwrite-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let result = overwrite(&root, "users/list.http", "GET https://api.example.com");

        assert!(matches!(result, Err(AppError::RequestNotFound(_))));
    }

    #[test]
    fn leaves_no_temporary_file_after_an_overwrite() {
        let root = workspace("overwrite-temp");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        overwrite(&root, "users/list.http", "POST https://api.example.com").unwrap();

        let entries: Vec<_> = fs::read_dir(collections_dir(&root).join("users"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(entries, vec!["list.http"]);
    }

    #[test]
    fn saves_into_a_nested_path() {
        let root = workspace("save-nested");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        fs::create_dir_all(collections_dir(&root).join("users/admin")).unwrap();

        let path = save(
            &root,
            "users/admin/delete.http",
            "GET https://api.example.com",
        )
        .unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn refuses_to_save_outside_the_workspace() {
        let root = workspace("save-escape");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = save(&root, "../../evil.http", "GET https://api.example.com");

        assert!(matches!(result, Err(AppError::InvalidRequestName(_))));
    }

    #[test]
    fn refuses_to_save_in_a_missing_collection() {
        let root = workspace("save-missing-collection");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = save(&root, "absent/list.http", "GET https://api.example.com");

        assert!(matches!(result, Err(AppError::MissingCollection(_))));
    }

    #[test]
    fn leaves_no_temporary_file_behind() {
        let root = workspace("save-temp");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        let entries: Vec<_> = fs::read_dir(collections_dir(&root).join("users"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        assert_eq!(entries, vec!["list.http"]);
    }

    #[test]
    fn a_saved_request_reads_back_identically() {
        let root = workspace("save-round-trip");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let source = "POST https://api.example.com/users\n\
                      Content-Type: application/json\n\n\
                      {\"name\": \"John\"}";
        let file = crate::http_file::parse(source)
            .and_then(Request::try_from)
            .unwrap();
        let text = crate::http_file::to_string(&file).unwrap();

        let path = save(&root, "users/create.http", &text).unwrap();

        let reread = crate::http_file::read(path.to_str().unwrap())
            .and_then(Request::try_from)
            .unwrap();

        assert_eq!(file.url, reread.url);
        assert_eq!(file.body, reread.body);
        assert_eq!(file.headers["content-type"], reread.headers["content-type"]);
    }

    // delete

    #[test]
    fn deletes_a_request() {
        let root = workspace("delete");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        let path = save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        delete(&root, "users/list.http").unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn deletes_without_the_extension() {
        let root = workspace("delete-no-ext");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        let path = save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        delete(&root, "users/list").unwrap();

        assert!(!path.exists());
    }

    #[test]
    fn reports_a_missing_request_on_delete() {
        let root = workspace("delete-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let result = delete(&root, "users/absent.http");

        assert!(matches!(result, Err(AppError::RequestNotFound(_))));
    }

    #[test]
    fn never_deletes_a_collection() {
        let root = workspace("delete-collection");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        // "users" has no extension, so it resolves to users.http, not users/.
        let result = delete(&root, "users");

        assert!(matches!(result, Err(AppError::RequestNotFound(_))));
        assert!(collections_dir(&root).join("users").is_dir());
    }

    // rename_request

    #[test]
    fn renames_a_request_and_keeps_its_content() {
        let root = workspace("rename-request");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        let source = "# Keep me\nGET https://api.example.com/users\n";
        let old = save(&root, "users/list.http", source).unwrap();

        let new = rename_request(&root, "users/list", "users/index").unwrap();

        assert_eq!(new, request_path(&root, "users/index.http").unwrap());
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new).unwrap(), source);
    }

    #[test]
    fn renaming_a_request_to_its_own_name_is_a_no_op() {
        let root = workspace("rename-request-same");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        let path = save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        let renamed = rename_request(&root, "users/list.http", "users/list").unwrap();

        assert_eq!(renamed, path);
        assert!(renamed.is_file());
    }

    #[test]
    fn refuses_to_rename_a_missing_request() {
        let root = workspace("rename-request-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let result = rename_request(&root, "users/absent.http", "users/index");

        assert!(matches!(result, Err(AppError::RequestNotFound(_))));
    }

    #[test]
    fn refuses_to_rename_onto_an_existing_request() {
        let root = workspace("rename-request-existing");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        save(&root, "users/list.http", "GET https://api.example.com").unwrap();
        save(&root, "users/index.http", "GET https://api.example.com").unwrap();

        let result = rename_request(&root, "users/list", "users/index");

        assert!(matches!(result, Err(AppError::RequestAlreadyExists(_))));
        assert!(collections_dir(&root).join("users/list.http").is_file());
        assert!(collections_dir(&root).join("users/index.http").is_file());
    }

    #[test]
    fn refuses_to_rename_a_request_into_a_missing_collection() {
        let root = workspace("rename-request-missing-collection");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        save(&root, "users/list.http", "GET https://api.example.com").unwrap();

        let result = rename_request(&root, "users/list", "absent/index");

        assert!(matches!(result, Err(AppError::MissingCollection(_))));
    }

    // new_request

    #[test]
    fn creates_a_request_from_the_template() {
        let root = workspace("new-request");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();

        let path = new_request(&root, "users/list.http").unwrap();

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("GET https://api.example.com"));
        assert!(content.contains("#"));
    }

    #[test]
    fn a_new_request_is_parsable() {
        let root = workspace("new-request-parsable");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        let path = new_request(&root, "users/list.http").unwrap();

        let request = crate::http_file::read(path.to_str().unwrap())
            .and_then(Request::try_from)
            .unwrap();

        assert_eq!(request.method, reqwest::Method::GET);
    }

    #[test]
    fn refuses_to_create_a_request_in_a_missing_collection() {
        let root = workspace("new-request-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = new_request(&root, "users/list.http");

        assert!(matches!(result, Err(AppError::MissingCollection(_))));
    }

    #[test]
    fn refuses_to_create_a_request_twice() {
        let root = workspace("new-request-twice");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_collection(&root, "users").unwrap();
        new_request(&root, "users/list.http").unwrap();

        let result = new_request(&root, "users/list.http");

        assert!(matches!(result, Err(AppError::RequestAlreadyExists(_))));
    }

    // request_path

    #[test]
    fn request_path_appends_the_extension_once() {
        let root = workspace("request-path");
        let _project = Project::new(&root);
        init(&root).unwrap();

        assert_eq!(
            request_path(&root, "users/list").unwrap(),
            request_path(&root, "users/list.http").unwrap()
        );
    }

    // environments

    fn write_environment(root: &Path, name: &str, content: &str) {
        let directory = environments_dir(root);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("{name}.env")), content).unwrap();
    }

    #[test]
    fn a_workspace_without_environments_lists_none() {
        // The pre-phase-11 layout: a valid workspace with no environments
        // directory at all.
        let root = workspace("env-no-dir");
        let _project = Project::new(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join(CONFIG_FILE),
            format!("{{\n  \"format\": {SUPPORTED_FORMAT}\n}}\n"),
        )
        .unwrap();

        assert!(list_environments(&root).unwrap().is_empty());
    }

    #[test]
    fn lists_environment_files_sorted() {
        let root = workspace("env-list");
        let _project = Project::new(&root);
        init(&root).unwrap();
        write_environment(&root, "prod", "a=1\n");
        write_environment(&root, "dev", "a=1\n");
        fs::write(environments_dir(&root).join("notes.txt"), "ignored").unwrap();

        assert_eq!(
            list_environments(&root).unwrap(),
            vec!["dev", "example", "prod"]
        );
    }

    #[test]
    fn reads_an_environment_file() {
        let root = workspace("env-read");
        let _project = Project::new(&root);
        init(&root).unwrap();
        write_environment(&root, "dev", "base_url=https://api.dev.example.com\n");

        let variables = read_environment(&root, "dev").unwrap();

        assert_eq!(variables["base_url"], "https://api.dev.example.com");
    }

    #[test]
    fn reports_a_missing_environment() {
        let root = workspace("env-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = read_environment(&root, "ghost");

        assert!(matches!(result, Err(AppError::EnvironmentNotFound(name)) if name == "ghost"));
    }

    #[test]
    fn rejects_an_environment_path_outside_the_directory() {
        let root = workspace("env-escape");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = read_environment(&root, "../secrets");

        assert!(matches!(result, Err(AppError::InvalidRequestName(_))));
    }

    #[test]
    fn stores_and_reads_the_active_environment() {
        let root = workspace("env-active");
        let _project = Project::new(&root);
        init(&root).unwrap();

        assert_eq!(active_environment(&root).unwrap(), None);

        set_active_environment(&root, "dev").unwrap();

        assert_eq!(active_environment(&root).unwrap(), Some("dev".to_string()));
        // The format key survives the rewrite.
        assert_eq!(read_format(&root).unwrap(), SUPPORTED_FORMAT);
    }

    #[test]
    fn init_seeds_the_example_environment_and_gitignore() {
        let root = workspace("env-init-seed");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let example = fs::read_to_string(environments_dir(&root).join("example.env")).unwrap();
        assert!(example.contains("base_url"));

        let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gitignore.contains("environments/*.env"));
        assert!(gitignore.contains("!environments/example.env"));
        // The seeded example must be selectable: it is a real file.
        assert_eq!(list_environments(&root).unwrap(), vec!["example"]);
    }

    #[test]
    fn init_extends_an_existing_gitignore() {
        let root = workspace("env-init-gitignore");
        let _project = Project::new(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".gitignore"), "target/\n").unwrap();

        init(&root).unwrap();

        let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(gitignore.starts_with("target/\n"));
        assert!(gitignore.contains("environments/*.env"));
    }

    // environment CRUD

    #[test]
    fn creates_an_environment_from_the_template() {
        let root = workspace("env-create");
        let _project = Project::new(&root);
        init(&root).unwrap();

        create_environment(&root, "staging").unwrap();

        let text = read_environment_text(&root, "staging").unwrap();
        assert!(text.contains("KEY = value"), "{text}");
        assert!(
            list_environments(&root)
                .unwrap()
                .contains(&"staging".to_string())
        );
    }

    #[test]
    fn refuses_to_create_an_existing_environment() {
        let root = workspace("env-create-twice");
        let _project = Project::new(&root);
        init(&root).unwrap();
        create_environment(&root, "dev").unwrap();

        let result = create_environment(&root, "dev");

        assert!(matches!(
            result,
            Err(AppError::EnvironmentAlreadyExists(name)) if name == "dev"
        ));
    }

    #[test]
    fn renames_an_environment() {
        let root = workspace("env-rename");
        let _project = Project::new(&root);
        init(&root).unwrap();
        write_environment(&root, "dev", "a=1\n");

        rename_environment(&root, "dev", "live").unwrap();

        assert!(read_environment(&root, "dev").is_err());
        assert_eq!(read_environment(&root, "live").unwrap()["a"], "1");
    }

    #[test]
    fn refuses_to_rename_onto_an_existing_environment() {
        let root = workspace("env-rename-existing");
        let _project = Project::new(&root);
        init(&root).unwrap();
        write_environment(&root, "dev", "a=1\n");
        write_environment(&root, "live", "b=2\n");

        let result = rename_environment(&root, "dev", "live");

        assert!(matches!(result, Err(AppError::EnvironmentAlreadyExists(_))));
        assert_eq!(read_environment(&root, "dev").unwrap()["a"], "1");
        assert_eq!(read_environment(&root, "live").unwrap()["b"], "2");
    }

    #[test]
    fn deletes_an_environment_and_reports_a_missing_one() {
        let root = workspace("env-delete");
        let _project = Project::new(&root);
        init(&root).unwrap();
        write_environment(&root, "dev", "a=1\n");

        delete_environment(&root, "dev").unwrap();

        assert!(matches!(
            delete_environment(&root, "dev"),
            Err(AppError::EnvironmentNotFound(_))
        ));
        assert!(
            list_environments(&root)
                .unwrap()
                .contains(&"example".to_string())
        );
    }

    #[test]
    fn environment_text_round_trips_comments_and_order() {
        let root = workspace("env-text");
        let _project = Project::new(&root);
        init(&root).unwrap();
        let original = "# my vars\nzeta = 1\nalpha = 2\n";
        write_environment(&root, "dev", original);

        let text = read_environment_text(&root, "dev").unwrap();
        overwrite_environment(&root, "dev", &text).unwrap();

        assert_eq!(read_environment_text(&root, "dev").unwrap(), original);
    }

    #[test]
    fn refuses_to_overwrite_a_missing_environment() {
        let root = workspace("env-overwrite-missing");
        let _project = Project::new(&root);
        init(&root).unwrap();

        let result = overwrite_environment(&root, "ghost", "a=1\n");

        assert!(matches!(result, Err(AppError::EnvironmentNotFound(_))));
    }

    #[test]
    fn clears_the_active_environment_keeping_the_format() {
        let root = workspace("env-clear-active");
        let _project = Project::new(&root);
        init(&root).unwrap();
        set_active_environment(&root, "dev").unwrap();

        clear_active_environment(&root).unwrap();

        assert_eq!(active_environment(&root).unwrap(), None);
        assert_eq!(read_format(&root).unwrap(), SUPPORTED_FORMAT);
        // Setting it again still works after the removal.
        set_active_environment(&root, "dev").unwrap();
        assert_eq!(active_environment(&root).unwrap(), Some("dev".to_string()));
    }
}

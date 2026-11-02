use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{ListState, Paragraph},
};
use ratatui_textarea::{CursorMove, TextArea};

use reqwest::Method;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use rully_core::{
    env_file, errors::AppError, http, http_file, request::Request, response::Response, store,
};
use std::collections::HashMap;

pub fn run(root: PathBuf) -> Result<(), AppError> {
    ratatui::run(|terminal| App::new(root).run(terminal)).map_err(AppError::Terminal)
}

mod keys;
use keys::Action;

mod left_panel;
use left_panel::LeftPanel;

mod env_panel;
use env_panel::EnvPanel;

mod view;

mod collection_prompt;

mod comments_editor;

mod methods_menu;

mod request_bar;

mod request_panel;
use request_panel::RequestTab;

mod response_panel;

mod content;
use content::{AUTH_KINDS, AuthKind, BODY_KINDS, BodyKind};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
struct App {
    // Workspace and collections tree (left panel).
    root: PathBuf,            // workspace root: the `rully/collections` directory
    items: Vec<String>, // rows shown in the tree: the collections, or the requests of the open collection
    error: Option<String>, // last error to show in the tree, if any
    state: ListState,   // tree selection: which row is highlighted
    location: Option<String>, // None = listing collections; Some(name) = listing that collection's requests

    // Loaded request (request bar + request panel).
    draft: Option<Draft>,    // the request being viewed, once one is loaded
    request_tab: RequestTab, // active request tab: Headers or Body

    send: SendState,
    client: reqwest::Client,
    response_tx: UnboundedSender<Result<Response, AppError>>,
    response_rx: UnboundedReceiver<Result<Response, AppError>>,
    pending: Option<Request>,

    // Environments section (below the tree).
    environments: Vec<String>,         // env names, file stems of `*.env`
    active_env: Option<String>,        // selected in rully.json, shown with `●`
    env_vars: HashMap<String, String>, // variables of the active env, resolved at send
    env_state: ListState,              // cursor inside the environments section

    edit_mode: EditMode,
    edit_input: TextArea<'static>,
    prompt: Option<Prompt>,

    // Focus and the menus that float over the interface.
    focus: Focus,
    method_menu: Menu, // the method picker: open flag + highlighted entry
    body_menu: Menu,   // the body type picker, anchored under the request panel
    auth_menu: Menu,   // the auth type picker, anchored under the request panel

    // Event loop.
    exit: bool,
}

/// Which panel currently receives keys. `Tab` cycles through them in the order
/// below, which follows their layout.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    /// The collections/requests tree on the left.
    LeftPanel,
    /// The method + URL bar at the top of the right column.
    RequestBar,
    /// The headers/body tabs of the loaded request.
    Request,
    /// The response panel (not populated yet).
    Response,
    /// The environments section below the tree.
    Environments,
}

/// The request being viewed: a decoded `.http` file held in memory. It is a
/// draft because the TUI will eventually edit it before saving.
#[derive(Debug)]
struct Draft {
    relative: Option<String>,
    comments: Vec<String>,
    method_index: usize,
    url: String,
    headers: HeaderMap,
    body: Option<String>,
    body_kind: BodyKind,
}

impl Draft {
    fn empty() -> Self {
        Self {
            relative: None,
            comments: Vec::new(),
            method_index: 0,
            url: String::new(),
            headers: HeaderMap::new(),
            body: None,
            body_kind: BodyKind::None,
        }
    }

    /// Applies a body type chosen in the UI to the draft.
    ///
    /// The raw body is kept as-is; only the type and its `Content-Type` change.
    /// Choosing `None` hides the body and omits it when the request is sent,
    /// but keeps both the body and its `Content-Type` in the draft.
    fn set_body_kind(&mut self, kind: BodyKind) {
        self.body_kind = kind;

        if let Some(content_type) = kind.content_type() {
            self.headers
                .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
        }
    }

    /// Applies an auth type chosen in the UI to the draft.
    ///
    /// Auth is derived from headers and the type is never stored in the file.
    fn set_auth_kind(&mut self, kind: AuthKind) {
        let current = AuthKind::of(&self.headers);
        if current == kind {
            return;
        }

        // Remove only known auth headers
        let known_scheme = self
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split_whitespace().next())
            .is_some_and(|scheme| {
                scheme.eq_ignore_ascii_case("bearer") || scheme.eq_ignore_ascii_case("basic")
            });

        if known_scheme {
            self.headers.remove(AUTHORIZATION);
        }
        self.headers.remove("x-api-key");

        match kind {
            AuthKind::None => {}
            AuthKind::Bearer => {
                self.headers
                    .insert(AUTHORIZATION, HeaderValue::from_static("Bearer"));
            }
            AuthKind::Basic => {
                self.headers
                    .insert(AUTHORIZATION, HeaderValue::from_static("Basic"));
            }
            AuthKind::ApiKey => {
                self.headers.insert(
                    HeaderName::from_static("x-api-key"),
                    HeaderValue::from_static(""),
                );
            }
        }
    }

    fn set_url(&mut self, url: &str) {
        self.url = url.trim().to_string();
    }

    fn set_body(&mut self, body: &str) -> Result<(), AppError> {
        if self.body_kind == BodyKind::Json && !body.trim().is_empty() {
            serde_json::from_str::<serde_json::Value>(body).map_err(AppError::InvalidJson)?;
        }

        self.body = if body.is_empty() {
            None
        } else {
            Some(body.to_string())
        };
        self.refresh_body_kind();
        Ok(())
    }

    fn set_comments(&mut self, text: &str) -> Result<(), AppError> {
        if text.is_empty() {
            self.comments.clear();
            return Ok(());
        }

        let lines: Vec<String> = text.split('\n').map(str::to_string).collect();

        for (index, line) in lines.iter().enumerate() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                return Err(AppError::InvalidHttpFile(format!(
                    "line {}: comments must start with #",
                    index + 1
                )));
            }
        }

        self.comments = lines;
        Ok(())
    }

    fn comments_text(&self) -> String {
        self.comments.join("\n")
    }

    /// Builds the request to send, resolving `{{name}}` placeholders against
    /// `vars` first — an unresolved name fails here, before URL parsing, so
    /// a template never leaves the machine half-substituted.
    fn to_request(&self, vars: &HashMap<String, String>) -> Result<Request, AppError> {
        let url = env_file::interpolate(&self.url, vars)?;

        let mut headers = HeaderMap::new();
        for (name, value) in &self.headers {
            let raw = value
                .to_str()
                .map_err(|_| AppError::InvalidHeader(format!("non-UTF8 value: {name}")))?;
            let resolved = env_file::interpolate(raw, vars)?;
            let resolved = HeaderValue::from_str(&resolved).map_err(|_| {
                AppError::InvalidHeader(format!("Invalid header value after interpolation: {name}"))
            })?;
            headers.append(name.clone(), resolved);
        }

        let body = self
            .body
            .as_deref()
            .map(|body| env_file::interpolate(body, vars))
            .transpose()?;

        let method = METHODS[self.method_index];
        let method = Method::from_bytes(method.as_bytes())
            .map_err(|_| AppError::InvalidHttpFile(format!("unsupported method: {method}")))?;

        let mut request = Request::new(method, &url)?;
        request.headers = headers;
        request.body = body;

        Ok(request)
    }

    /// Serialises the draft back to the file format, placeholders included.
    ///
    /// The URL is written raw on purpose: `{{base_url}}/users` is a valid
    /// template but not a parseable URL, and saving must never interpolate —
    /// that happens at send time only.
    fn source(&self) -> Result<String, AppError> {
        let mut source = String::new();

        for line in &self.comments {
            source.push_str(line);
            source.push('\n');
        }

        source.push_str(&http_file::to_string_parts(
            METHODS[self.method_index],
            &self.url,
            &self.headers,
            self.body.as_deref(),
        )?);

        Ok(source)
    }

    fn set_headers_from_text(&mut self, text: &str) -> Result<(), AppError> {
        self.headers = content::parse_headers(text)?;
        self.refresh_body_kind();
        Ok(())
    }

    fn headers_text(&self) -> Result<String, AppError> {
        content::headers_text(&self.headers)
    }

    /// The editable auth value: the token for Bearer, `user:pass` for Basic,
    /// and the key itself for API key. `None` means there is no auth to edit.
    fn auth_value(&self) -> Option<String> {
        let authorization = self
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        match AuthKind::of(&self.headers) {
            AuthKind::None => None,
            AuthKind::Bearer => Some(
                authorization
                    .and_then(|value| value.split_once(' '))
                    .map(|(_, token)| token.trim().to_string())
                    .unwrap_or_default(),
            ),
            AuthKind::Basic => Some(
                authorization
                    .and_then(|value| value.split_once(' '))
                    .and_then(|(_, token)| STANDARD.decode(token).ok())
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .unwrap_or_default(),
            ),
            AuthKind::ApiKey => Some(
                self.headers
                    .get("x-api-key")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_string(),
            ),
        }
    }

    fn set_auth_value(&mut self, value: &str) -> Result<(), AppError> {
        let value = value.trim();

        let header = match AuthKind::of(&self.headers) {
            AuthKind::None => return Ok(()),
            AuthKind::Bearer => {
                if value.is_empty() {
                    "Bearer".to_string()
                } else {
                    format!("Bearer {value}")
                }
            }
            AuthKind::Basic => {
                if value.is_empty() {
                    "Basic".to_string()
                } else if value.contains(':') {
                    format!("Basic {}", STANDARD.encode(value))
                } else {
                    return Err(AppError::InvalidHeader(
                        "Basic credentials must be user:password".to_string(),
                    ));
                }
            }
            AuthKind::ApiKey => {
                let header_value = HeaderValue::from_str(value)
                    .map_err(|_| AppError::InvalidHeader("invalid API key value".to_string()))?;
                self.headers
                    .insert(HeaderName::from_static("x-api-key"), header_value);
                return Ok(());
            }
        };

        let header_value = HeaderValue::from_str(&header)
            .map_err(|_| AppError::InvalidHeader("invalid authorization value".to_string()))?;
        self.headers.insert(AUTHORIZATION, header_value);
        Ok(())
    }

    /// Reclassifies the body after an edit changed its presence or `Content-Type`.
    /// An explicit `None` stays hidden.
    fn refresh_body_kind(&mut self) {
        if self.body_kind != BodyKind::None {
            self.body_kind = BodyKind::of(&self.headers, self.body.as_deref());
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum SendState {
    Idle,
    Sending,
    Done(Response),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum EditMode {
    None,
    Comments,
    Url,
    Headers,
    Body,
    Auth,
    Environment,
}

impl EditMode {
    /// Whether the editor accepts `Enter` as a line break instead of ignoring it.
    fn allows_newline(self) -> bool {
        matches!(
            self,
            Self::Comments | Self::Headers | Self::Body | Self::Environment
        )
    }
}

/// A modal list shown over the interface. While it is open it captures the
/// keys, so the focus underneath never moves.
#[derive(Debug)]
struct Menu {
    open: bool,
    index: usize,
}

impl Menu {
    fn new() -> Self {
        Self {
            open: false,
            index: 0,
        }
    }

    fn show(&mut self, index: usize) {
        self.index = index;
        self.open = true;
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn previous(&mut self, len: usize) {
        self.index = (self.index + len - 1) % len;
    }

    fn next(&mut self, len: usize) {
        self.index = (self.index + 1) % len;
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PromptKind {
    NewCollection,
    RenameCollection { old: String },
    DeleteCollection { name: String },
    NewRequest { collection: String },
    RenameRequest { collection: String, old: String },
    DeleteRequest { collection: String, name: String },
    NewEnvironment,
    RenameEnvironment { old: String },
    DeleteEnvironment { name: String },
}

#[derive(Debug)]
struct Prompt {
    kind: PromptKind,
    input: TextArea<'static>,
    error: Option<String>,
}

impl Prompt {
    fn new_collection() -> Self {
        Self {
            kind: PromptKind::NewCollection,
            input: TextArea::default(),
            error: None,
        }
    }

    fn rename_collection(name: &str) -> Self {
        let mut input = TextArea::new(vec![name.to_string()]);
        input.move_cursor(CursorMove::End);

        Self {
            kind: PromptKind::RenameCollection {
                old: name.to_string(),
            },
            input,
            error: None,
        }
    }

    fn delete_collection(name: &str) -> Self {
        Self {
            kind: PromptKind::DeleteCollection {
                name: name.to_string(),
            },
            input: TextArea::default(),
            error: None,
        }
    }

    fn new_request(collection: &str) -> Self {
        Self {
            kind: PromptKind::NewRequest {
                collection: collection.to_string(),
            },
            input: TextArea::default(),
            error: None,
        }
    }

    fn rename_request(collection: &str, name: &str) -> Self {
        let mut input = TextArea::new(vec![request_display_name(name).to_string()]);
        input.move_cursor(CursorMove::End);

        Self {
            kind: PromptKind::RenameRequest {
                collection: collection.to_string(),
                old: name.to_string(),
            },
            input,
            error: None,
        }
    }

    fn delete_request(collection: &str, name: &str) -> Self {
        Self {
            kind: PromptKind::DeleteRequest {
                collection: collection.to_string(),
                name: name.to_string(),
            },
            input: TextArea::default(),
            error: None,
        }
    }

    fn new_environment() -> Self {
        Self {
            kind: PromptKind::NewEnvironment,
            input: TextArea::default(),
            error: None,
        }
    }

    fn rename_environment(name: &str) -> Self {
        let mut input = TextArea::new(vec![name.to_string()]);
        input.move_cursor(CursorMove::End);

        Self {
            kind: PromptKind::RenameEnvironment {
                old: name.to_string(),
            },
            input,
            error: None,
        }
    }

    fn delete_environment(name: &str) -> Self {
        Self {
            kind: PromptKind::DeleteEnvironment {
                name: name.to_string(),
            },
            input: TextArea::default(),
            error: None,
        }
    }

    fn accepts_text(&self) -> bool {
        !matches!(
            &self.kind,
            PromptKind::DeleteCollection { .. }
                | PromptKind::DeleteRequest { .. }
                | PromptKind::DeleteEnvironment { .. }
        )
    }

    fn title(&self) -> &'static str {
        match &self.kind {
            PromptKind::NewCollection => "New collection",
            PromptKind::RenameCollection { .. } => "Rename collection",
            PromptKind::DeleteCollection { .. } => "Delete collection",
            PromptKind::NewRequest { .. } => "New request",
            PromptKind::RenameRequest { .. } => "Rename request",
            PromptKind::DeleteRequest { .. } => "Delete request",
            PromptKind::NewEnvironment => "New environment",
            PromptKind::RenameEnvironment { .. } => "Rename environment",
            PromptKind::DeleteEnvironment { .. } => "Delete environment",
        }
    }

    fn hint(&self) -> &'static str {
        if self.accepts_text() {
            "Enter save · Esc cancel"
        } else {
            "Enter delete · Esc cancel"
        }
    }
}

fn prompt_collection_name(input: &TextArea<'_>) -> Result<String, AppError> {
    let name = input.lines().join("").trim().to_string();

    if name.is_empty() {
        return Err(AppError::InvalidRequestName(
            "collection name cannot be empty".to_string(),
        ));
    }

    if name.contains('/') {
        return Err(AppError::InvalidRequestName(
            "sub-collections are not supported yet".to_string(),
        ));
    }

    Ok(name)
}

fn prompt_request_name(input: &TextArea<'_>) -> Result<String, AppError> {
    let typed = input.lines().join("").trim().to_string();

    if typed.is_empty() {
        return Err(AppError::InvalidRequestName(
            "request name cannot be empty".to_string(),
        ));
    }

    if typed.contains('/') {
        return Err(AppError::InvalidRequestName(
            "request names cannot contain '/'".to_string(),
        ));
    }

    let name = typed.strip_suffix(".http").unwrap_or(&typed);

    if name.is_empty() {
        return Err(AppError::InvalidRequestName(
            "request name cannot be empty".to_string(),
        ));
    }

    store::validate_name(name)?;

    Ok(format!("{name}.http"))
}

fn request_display_name(name: &str) -> &str {
    name.strip_suffix(".http").unwrap_or(name)
}

fn prompt_environment_name(input: &TextArea<'_>) -> Result<String, AppError> {
    let typed = input.lines().join("").trim().to_string();

    if typed.contains('/') {
        return Err(AppError::InvalidRequestName(
            "environment names cannot contain '/'".to_string(),
        ));
    }

    // The list shows file stems, so `dev.env` and `dev` name the same file.
    let name = typed.strip_suffix(".env").unwrap_or(&typed).to_string();

    if name.is_empty() {
        return Err(AppError::InvalidRequestName(
            "environment name cannot be empty".to_string(),
        ));
    }

    Ok(name)
}

const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];
const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(50);

impl App {
    /// Runs the draw/read-keys loop until `quit` is dispatched.
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.spawn_pending();
            self.receive_responses();
        }
        Ok(())
    }

    fn new(root: PathBuf) -> Self {
        let (response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = Self {
            root,
            items: Vec::new(),
            error: None,
            state: ListState::default(),
            location: None,
            draft: Some(Draft::empty()),
            request_tab: RequestTab::Headers,
            focus: Focus::LeftPanel,
            method_menu: Menu::new(),
            body_menu: Menu::new(),
            auth_menu: Menu::new(),
            exit: false,
            edit_mode: EditMode::None,
            edit_input: TextArea::default(),
            prompt: None,
            send: SendState::Idle,
            client: http::default_client().expect("reqwest client builds without network access"),
            response_tx,
            response_rx,
            pending: None,
            environments: Vec::new(),
            active_env: None,
            env_vars: HashMap::new(),
            env_state: ListState::default(),
        };
        app.refresh();
        app.refresh_environments();
        app
    }

    /// Reloads `items` for the current level: the collections at the root, or
    /// the requests of the open collection.
    fn refresh(&mut self) {
        self.refresh_selecting(None);
    }

    fn refresh_selecting(&mut self, selected: Option<&str>) {
        let previous = selected
            .map(str::to_string)
            .or_else(|| self.selected_entry().map(str::to_string));

        let result = match &self.location {
            None => store::list_collections(&self.root),
            Some(name) => store::list_requests(&self.root, name).map(|entries| {
                entries
                    .into_iter()
                    .filter(|entry| http_file::is_request_path(entry))
                    .collect()
            }),
        };

        match result {
            Ok(names) => {
                let index = previous
                    .as_deref()
                    .and_then(|name| names.iter().position(|entry| entry == name))
                    .unwrap_or(0);

                self.state.select(if names.is_empty() {
                    None
                } else {
                    Some(index.min(names.len() - 1))
                });
                self.items = names;
                self.error = None;
            }
            Err(error) => {
                self.items.clear();
                self.error = Some(error.to_string());
                self.state.select(None);
            }
        }
    }

    /// Reloads the environments list, the selected environment and its
    /// variables. A workspace created before environments existed simply
    /// lists none; a selection pointing at a missing file falls back to no
    /// active environment and reports the problem.
    fn refresh_environments(&mut self) {
        match store::list_environments(&self.root) {
            Ok(names) => self.environments = names,
            Err(error) => {
                self.environments.clear();
                self.error = Some(error.to_string());
                return;
            }
        }

        let selected = match store::active_environment(&self.root) {
            Ok(selected) => selected,
            Err(error) => {
                self.clear_environment();
                self.error = Some(error.to_string());
                return;
            }
        };

        let Some(name) = selected else {
            self.clear_environment();
            return;
        };

        match store::read_environment(&self.root, &name) {
            Ok(variables) => {
                self.active_env = Some(name.clone());
                self.env_vars = variables;
                let cursor = self
                    .environments
                    .iter()
                    .position(|entry| *entry == name)
                    .or_else(|| (!self.environments.is_empty()).then_some(0));
                self.env_state.select(cursor);
            }
            Err(error) => {
                self.clear_environment();
                self.error = Some(error.to_string());
            }
        }
    }

    /// Drops the active environment, leaving the cursor on the first entry.
    fn clear_environment(&mut self) {
        self.active_env = None;
        self.env_vars = HashMap::new();
        self.env_state
            .select((!self.environments.is_empty()).then_some(0));
    }

    fn select_next_environment(&mut self) {
        let len = self.environments.len();
        if len == 0 {
            return;
        }

        let index = self
            .env_state
            .selected()
            .map_or(0, |index| (index + 1) % len);
        self.env_state.select(Some(index));
    }

    fn select_previous_environment(&mut self) {
        let len = self.environments.len();
        if len == 0 {
            return;
        }

        let index =
            self.env_state.selected().map_or(
                len - 1,
                |index| if index == 0 { len - 1 } else { index - 1 },
            );
        self.env_state.select(Some(index));
    }

    /// Loads the highlighted environment, makes it active and persists the
    /// choice in `rully.json`.
    fn activate_selected_environment(&mut self) {
        let Some(index) = self.env_state.selected() else {
            return;
        };
        let Some(name) = self.environments.get(index).cloned() else {
            return;
        };

        match store::read_environment(&self.root, &name) {
            Ok(variables) => {
                self.active_env = Some(name.clone());
                self.env_vars = variables;
                self.error = store::set_active_environment(&self.root, &name)
                    .err()
                    .map(|error| error.to_string());
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// The environment under the cursor in the environments section.
    fn selected_environment_name(&self) -> Option<String> {
        let index = self.env_state.selected()?;
        self.environments.get(index).cloned()
    }

    /// Moves the cursor onto `name` after a refresh, keeping the previous
    /// position when the name is gone.
    fn select_environment_cursor(&mut self, name: &str) {
        if let Some(index) = self.environments.iter().position(|entry| entry == name) {
            self.env_state.select(Some(index));
        }
    }

    /// The entry that should take the cursor after `name` is deleted: the
    /// next one, or the previous when deleting the last.
    fn environment_neighbor(&self, name: &str) -> Option<String> {
        let index = self.environments.iter().position(|entry| entry == name)?;
        self.environments
            .get(index + 1)
            .or_else(|| {
                index
                    .checked_sub(1)
                    .and_then(|previous| self.environments.get(previous))
            })
            .cloned()
    }

    fn draw(&mut self, frame: &mut Frame) {
        let title = view::bread_crumb(self.location.as_deref());
        let empty = view::empty_message(self.location.as_deref());

        let [main, help] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
        let [tree_column, detail] =
            Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)]).areas(main);

        // The environments section is a fixed strip under the tree: borders
        // included, one row per environment, clamped so it never eats the tree.
        let env_height = (self.environments.len() + 2).clamp(3, 8) as u16;
        let [tree, environments] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(env_height)])
                .areas(tree_column);

        let [bar, request, response] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ])
        .areas(detail);

        frame.render_stateful_widget(
            LeftPanel {
                items: &self.items,
                error: self.error.as_deref(),
                title: &title,
                empty,
                focused: self.focus == Focus::LeftPanel,
            },
            tree,
            &mut self.state,
        );

        frame.render_stateful_widget(
            EnvPanel {
                environments: &self.environments,
                active: self.active_env.as_deref(),
                focused: self.focus == Focus::Environments,
            },
            environments,
            &mut self.env_state,
        );

        let draft = self.draft.as_ref();
        request_bar::render(
            frame,
            bar,
            draft,
            self.focus == Focus::RequestBar,
            &self.edit_input,
            self.edit_mode,
        );
        request_panel::render(
            frame,
            request,
            draft,
            self.request_tab,
            self.focus == Focus::Request,
            self.edit_mode,
            &self.edit_input,
        );
        response_panel::render(frame, response, self.focus == Focus::Response, &self.send);

        let menu_open = self.method_menu.open || self.body_menu.open || self.auth_menu.open;

        frame.render_widget(
            Paragraph::new(keys::help_line(keys::current_binding(
                self.focus,
                self.location.as_deref(),
                menu_open,
                self.edit_mode,
            ))),
            help,
        );

        if self.method_menu.open {
            // Anchored under the method cell, in the bar's left border.
            let popup = Rect {
                x: bar.x,
                y: bar.y + 2,
                width: 8,
                height: 7,
            };
            methods_menu::render(frame, popup, &METHODS, self.method_menu.index);
        }

        if self.body_menu.open {
            let labels = BODY_KINDS.map(BodyKind::label);
            let popup = Rect {
                x: request.x + 1,
                y: request.y + 2,
                width: 12,
                height: 6,
            };
            methods_menu::render(frame, popup, &labels, self.body_menu.index);
        }

        if self.auth_menu.open {
            let labels = AUTH_KINDS.map(AuthKind::label);
            let popup = Rect {
                x: request.x + 1,
                y: request.y + 2,
                width: 12,
                height: 6,
            };
            methods_menu::render(frame, popup, &labels, self.auth_menu.index);
        }

        if self.edit_mode == EditMode::Comments {
            let area = frame.area();
            comments_editor::render(frame, area, &self.edit_input, "Comments", None);
        }

        if self.edit_mode == EditMode::Environment {
            let area = frame.area();
            let title = match self.selected_environment_name() {
                Some(name) => format!("Environment: {name}"),
                None => "Environment".to_string(),
            };
            let footer = Some(match &self.error {
                Some(error) => Line::from(error.clone()).style(Style::new().fg(Color::Red)),
                None => Line::from("ctrl+s save · esc cancel").style(view::dim()),
            });
            comments_editor::render(frame, area, &self.edit_input, &title, footer);
        }

        if let Some(prompt) = &self.prompt {
            let area = frame.area();
            collection_prompt::render(frame, area, prompt);
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if !event::poll(EVENT_POLL_TIMEOUT)? {
            return Ok(());
        }

        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    /// Looks the key up in the table for the current focus (or the method menu
    /// when it is open) and dispatches the matching action.
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.prompt.is_some() {
            self.handle_prompt_key(key_event);
            return;
        }

        let menu_open = self.method_menu.open || self.body_menu.open || self.auth_menu.open;
        let editing = self.edit_mode != EditMode::None;

        let bindings = keys::current_binding(
            self.focus,
            self.location.as_deref(),
            menu_open,
            self.edit_mode,
        );

        if let Some(binding) = bindings
            .iter()
            .find(|b| b.key == key_event.code && b.modifiers == key_event.modifiers)
        {
            self.dispatch(binding.action);
            return;
        }

        if editing {
            if key_event.code == KeyCode::Enter && self.edit_mode == EditMode::Comments {
                self.edit_input.insert_str("\n# ");
                return;
            }
            if key_event.code == KeyCode::Enter && !self.edit_mode.allows_newline() {
                return;
            }
            self.edit_input.input(key_event);
        }
    }

    fn handle_prompt_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.prompt = None;
                return;
            }
            KeyCode::Enter => {
                self.submit_prompt();
                return;
            }
            _ => {}
        }

        if let Some(prompt) = self.prompt.as_mut()
            && prompt.accepts_text()
        {
            prompt.input.input(key_event);
        }
    }

    fn submit_prompt(&mut self) {
        let Some(mut prompt) = self.prompt.take() else {
            return;
        };

        match prompt.kind.clone() {
            PromptKind::NewCollection => {
                let name = match prompt_collection_name(&prompt.input) {
                    Ok(name) => name,
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return;
                    }
                };

                match store::create_collection(&self.root, &name) {
                    Ok(()) => {
                        self.error = None;
                        self.refresh_selecting(Some(&format!("{name}/")));
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::RenameCollection { old } => {
                let name = match prompt_collection_name(&prompt.input) {
                    Ok(name) => name,
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return;
                    }
                };

                match store::rename_collection(&self.root, &old, &name) {
                    Ok(_) => {
                        self.error = None;
                        self.rebase_draft(&old, &name);
                        self.refresh_selecting(Some(&format!("{name}/")));
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::DeleteCollection { name } => {
                let neighbor = self.delete_neighbor();

                match store::delete_collection(&self.root, &name) {
                    Ok(()) => {
                        self.error = None;
                        self.refresh_selecting(neighbor.as_deref());
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::NewRequest { collection } => {
                let name = match prompt_request_name(&prompt.input) {
                    Ok(name) => name,
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return;
                    }
                };

                match store::new_request(&self.root, &format!("{collection}/{name}")) {
                    Ok(_) => {
                        self.error = None;
                        self.refresh_selecting(Some(&name));
                        self.load_selected_request();
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::RenameRequest { collection, old } => {
                let name = match prompt_request_name(&prompt.input) {
                    Ok(name) => name,
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return;
                    }
                };

                let old_relative = format!("{collection}/{old}");
                let new_relative = format!("{collection}/{name}");

                match store::rename_request(&self.root, &old_relative, &new_relative) {
                    Ok(_) => {
                        self.error = None;
                        self.rebase_draft(&old_relative, &new_relative);
                        self.refresh_selecting(Some(&name));
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::DeleteRequest { collection, name } => {
                let relative = format!("{collection}/{name}");
                let neighbor = self.delete_neighbor();
                let deleting_loaded = self
                    .draft
                    .as_ref()
                    .and_then(|draft| draft.relative.as_deref())
                    == Some(relative.as_str());

                match store::delete(&self.root, &relative) {
                    Ok(()) => {
                        self.error = None;

                        if deleting_loaded {
                            self.cancel_edit();
                            self.draft = Some(Draft::empty());
                        }

                        self.refresh_selecting(neighbor.as_deref());
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::NewEnvironment => {
                let name = match prompt_environment_name(&prompt.input) {
                    Ok(name) => name,
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return;
                    }
                };

                match store::create_environment(&self.root, &name) {
                    Ok(_) => {
                        self.error = None;
                        self.refresh_environments();
                        self.select_environment_cursor(&name);
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::RenameEnvironment { old } => {
                let name = match prompt_environment_name(&prompt.input) {
                    Ok(name) => name,
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                        return;
                    }
                };

                match store::rename_environment(&self.root, &old, &name) {
                    Ok(_) => {
                        self.error = None;

                        // The active environment follows the rename, in the
                        // TUI and in `rully.json` alike.
                        if self.active_env.as_deref() == Some(old.as_str()) {
                            self.active_env = Some(name.clone());
                            if let Err(error) = store::set_active_environment(&self.root, &name) {
                                self.error = Some(error.to_string());
                            }
                        }

                        self.refresh_environments();
                        self.select_environment_cursor(&name);
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
            PromptKind::DeleteEnvironment { name } => {
                let was_active = self.active_env.as_deref() == Some(name.as_str());
                let neighbor = self.environment_neighbor(&name);

                match store::delete_environment(&self.root, &name) {
                    Ok(()) => {
                        self.error = None;

                        if was_active {
                            self.active_env = None;
                            self.env_vars = HashMap::new();
                            if let Err(error) = store::clear_active_environment(&self.root) {
                                self.error = Some(error.to_string());
                            }
                        }

                        self.refresh_environments();
                        if let Some(neighbor) = neighbor {
                            self.select_environment_cursor(&neighbor);
                        }
                    }
                    Err(error) => {
                        prompt.error = Some(error.to_string());
                        self.prompt = Some(prompt);
                    }
                }
            }
        }
    }

    fn open_new_collection_prompt(&mut self) {
        if self.location.is_none() {
            self.prompt = Some(Prompt::new_collection());
        }
    }

    fn open_rename_collection_prompt(&mut self) {
        if let Some(name) = self.selected_collection() {
            self.prompt = Some(Prompt::rename_collection(name));
        }
    }

    fn open_delete_collection_prompt(&mut self) {
        if let Some(name) = self.selected_collection() {
            self.prompt = Some(Prompt::delete_collection(name));
        }
    }

    fn open_new_request_prompt(&mut self) {
        if let Some(collection) = self.location.clone() {
            self.prompt = Some(Prompt::new_request(&collection));
        }
    }

    fn open_rename_request_prompt(&mut self) {
        let Some(collection) = self.location.clone() else {
            return;
        };
        let Some(entry) = self.selected_entry().map(str::to_string) else {
            return;
        };

        if http_file::is_request_path(&entry) {
            self.prompt = Some(Prompt::rename_request(&collection, &entry));
        }
    }

    fn open_delete_request_prompt(&mut self) {
        let Some(collection) = self.location.clone() else {
            return;
        };
        let Some(entry) = self.selected_entry().map(str::to_string) else {
            return;
        };

        if http_file::is_request_path(&entry) {
            self.prompt = Some(Prompt::delete_request(&collection, &entry));
        }
    }

    fn open_rename_environment_prompt(&mut self) {
        if let Some(name) = self.selected_environment_name() {
            self.prompt = Some(Prompt::rename_environment(&name));
        }
    }

    fn open_delete_environment_prompt(&mut self) {
        if let Some(name) = self.selected_environment_name() {
            self.prompt = Some(Prompt::delete_environment(&name));
        }
    }

    /// Opens the selected environment's file as raw text in the modal editor,
    /// so comments and variable order survive a save.
    fn open_environment_editor(&mut self) {
        let Some(name) = self.selected_environment_name() else {
            return;
        };

        match store::read_environment_text(&self.root, &name) {
            Ok(text) => {
                let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
                if lines.is_empty() {
                    lines.push(String::new());
                }

                self.error = None;
                self.edit_input = TextArea::new(lines);
                self.edit_mode = EditMode::Environment;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn selected_collection(&self) -> Option<&str> {
        if self.location.is_some() {
            return None;
        }

        self.selected_entry()
            .and_then(|entry| entry.strip_suffix('/'))
    }

    fn delete_neighbor(&self) -> Option<String> {
        let index = self.state.selected()?;
        let neighbor = self.items.get(index + 1).or_else(|| {
            index
                .checked_sub(1)
                .and_then(|previous| self.items.get(previous))
        })?;

        neighbor.strip_suffix('/').map(str::to_string)
    }

    fn rebase_draft(&mut self, old: &str, new: &str) {
        if self.location.as_deref() == Some(old) {
            self.location = Some(new.to_string());
        }

        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let Some(relative) = draft.relative.clone() else {
            return;
        };

        if relative == old {
            draft.relative = Some(new.to_string());
            return;
        }

        let Some(rest) = relative
            .strip_prefix(old)
            .and_then(|rest| rest.strip_prefix('/'))
        else {
            return;
        };

        draft.relative = Some(format!("{new}/{rest}"));
    }

    fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit(),
            Action::SelectPrevious => self.select_previous(),
            Action::SelectNext => self.select_next(),
            Action::Activate => self.activate(),
            Action::GoBack => self.go_back(),
            Action::NewCollection => self.open_new_collection_prompt(),
            Action::RenameCollection => self.open_rename_collection_prompt(),
            Action::DeleteCollection => self.open_delete_collection_prompt(),
            Action::NewRequest => self.open_new_request_prompt(),
            Action::RenameRequest => self.open_rename_request_prompt(),
            Action::DeleteRequest => self.open_delete_request_prompt(),

            Action::NextFocus => self.next_focus(),
            Action::MenuPrevious => self.menu_previous(),
            Action::MenuNext => self.menu_next(),
            Action::MenuConfirm => self.menu_confirm(),
            Action::MenuCancel => self.menu_cancel(),

            Action::EnvPrevious => self.select_previous_environment(),
            Action::EnvNext => self.select_next_environment(),
            Action::EnvSelect => self.activate_selected_environment(),
            Action::NewEnvironment => self.prompt = Some(Prompt::new_environment()),
            Action::RenameEnvironment => self.open_rename_environment_prompt(),
            Action::DeleteEnvironment => self.open_delete_environment_prompt(),
            Action::EditEnvironment => self.open_environment_editor(),

            Action::TabPrevious => self.switch_tab(false),
            Action::TabNext => self.switch_tab(true),
            Action::OpenMethodMenu => self.open_method_menu(),
            Action::OpenBodyMenu => self.open_body_menu(),
            Action::OpenAuthMenu => self.open_auth_menu(),
            Action::FocusLeftPanel => self.focus = Focus::LeftPanel,

            Action::EditUrl => self.start_edit(EditMode::Url),
            Action::EditCurrent => self.edit_current_tab(),
            Action::EditComments => self.start_edit(EditMode::Comments),
            Action::Save => self.save_current_request(),
            Action::SubmitEdit => self.submit_edit(),
            Action::CancelEdit => self.cancel_edit(),

            Action::Send => self.send_request(),
        }
    }

    /// Moves the list selection down, wrapping at the end.
    fn select_next(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let next = self.state.selected().map_or(0, |i| (i + 1) % len);
        self.state.select(Some(next));
    }

    /// Moves the list selection up, wrapping at the start.
    fn select_previous(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let previous = self
            .state
            .selected()
            .map_or(len - 1, |i| (i + len - 1) % len);
        self.state.select(Some(previous));
    }

    /// The entry highlighted in the tree, if any.
    fn selected_entry(&self) -> Option<&str> {
        let index = self.state.selected()?;
        self.items.get(index).map(String::as_str)
    }

    /// Reads and decodes a `.http` file into a `Draft`.
    ///
    /// The URL is kept exactly as written — `Request::try_from` would parse it
    /// and reject `{{placeholder}}` templates, which are only resolved when
    /// the request is sent.
    fn read_draft(root: &Path, relative: &str) -> Result<Draft, AppError> {
        let path = store::request_path(root, relative)?;
        let file = http_file::read(&path.to_string_lossy())?;
        let comments = file.comments().to_vec();
        let headers = http_file::headers_from_pairs(file.headers())?;
        let body_kind = BodyKind::of(&headers, file.body());
        let method_index = METHODS
            .iter()
            .position(|method| *method == file.method())
            .unwrap_or(0);

        Ok(Draft {
            relative: Some(relative.to_string()),
            comments,
            method_index,
            url: file.url().to_string(),
            headers,
            body: file.body().map(str::to_string),
            body_kind,
        })
    }

    /// Loads the highlighted tree entry into `draft` when it is a request file.
    fn load_selected_request(&mut self) {
        let (Some(collection), Some(entry)) = (&self.location, self.selected_entry()) else {
            return;
        };
        if !http_file::is_request_path(entry) {
            return;
        }

        match Self::read_draft(&self.root, &format!("{collection}/{entry}")) {
            Ok(draft) => {
                self.cancel_edit();
                self.draft = Some(draft);
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Enter on the tree: opens the highlighted collection, or loads the
    /// highlighted request.
    fn activate(&mut self) {
        if self.location.is_some() {
            self.load_selected_request();
            return;
        }
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if let Some(name) = entry.strip_suffix('/') {
            self.location = Some(name.to_string());
            self.refresh();
        }
    }

    /// Steps back up from a collection to the list of collections.
    fn go_back(&mut self) {
        self.cancel_edit();
        if self.location.take().is_some() {
            self.refresh();
        }
    }

    fn quit(&mut self) {
        self.exit = true;
    }

    /// `Tab`: moves focus to the next panel, in layout order.
    fn next_focus(&mut self) {
        self.focus = match self.focus {
            Focus::LeftPanel => Focus::RequestBar,
            Focus::RequestBar => Focus::Request,
            Focus::Request => Focus::Response,
            Focus::Response => Focus::Environments,
            Focus::Environments => Focus::LeftPanel,
        };
    }

    fn open_method_menu(&mut self) {
        let index = self.draft.as_ref().map_or(0, |draft| draft.method_index);

        self.body_menu.close();
        self.method_menu.show(index);
    }

    fn open_body_menu(&mut self) {
        let Some(index) = self.draft.as_ref().map(|draft| draft.body_kind.index()) else {
            return;
        };

        self.method_menu.close();
        self.auth_menu.close();
        self.body_menu.show(index);
    }

    fn open_auth_menu(&mut self) {
        let Some(draft) = self.draft.as_ref() else {
            return;
        };
        let index = AuthKind::of(&draft.headers).index();

        self.method_menu.close();
        self.body_menu.close();
        self.auth_menu.show(index);
    }

    fn menu_previous(&mut self) {
        if self.method_menu.open {
            self.method_menu.previous(METHODS.len());
        } else if self.body_menu.open {
            self.body_menu.previous(BODY_KINDS.len());
        } else if self.auth_menu.open {
            self.auth_menu.previous(AUTH_KINDS.len());
        }
    }

    fn menu_next(&mut self) {
        if self.method_menu.open {
            self.method_menu.next(METHODS.len());
        } else if self.body_menu.open {
            self.body_menu.next(BODY_KINDS.len());
        } else if self.auth_menu.open {
            self.auth_menu.next(AUTH_KINDS.len());
        }
    }

    fn menu_confirm(&mut self) {
        if self.method_menu.open {
            let method_index = self.method_menu.index;
            self.draft.get_or_insert_with(Draft::empty).method_index = method_index;
            self.method_menu.close();
        } else if self.body_menu.open {
            let kind = BODY_KINDS[self.body_menu.index];
            if let Some(draft) = &mut self.draft {
                draft.set_body_kind(kind);
            }
            self.body_menu.close();
        } else if self.auth_menu.open {
            let kind = AUTH_KINDS[self.auth_menu.index];
            if let Some(draft) = &mut self.draft {
                draft.set_auth_kind(kind);
            }
            self.auth_menu.close();
        }
    }

    fn menu_cancel(&mut self) {
        self.method_menu.close();
        self.body_menu.close();
        self.auth_menu.close();
    }

    fn switch_tab(&mut self, forward: bool) {
        self.request_tab = RequestTab::switch_tabs(self.request_tab, forward);
    }

    fn edit_current_tab(&mut self) {
        let mode = match self.request_tab {
            RequestTab::Headers => EditMode::Headers,
            RequestTab::Body => EditMode::Body,
            RequestTab::Auth => EditMode::Auth,
        };
        self.start_edit(mode);
    }

    fn start_edit(&mut self, mode: EditMode) {
        let Some(draft) = self.draft.as_ref() else {
            return;
        };

        let text = match mode {
            EditMode::None => return,
            EditMode::Comments if draft.comments.is_empty() => Ok("# ".to_string()),
            EditMode::Comments => Ok(draft.comments_text()),
            EditMode::Url => Ok(draft.url.clone()),
            EditMode::Body if draft.body_kind == BodyKind::None => return,
            EditMode::Body => {
                let body = draft.body.clone().unwrap_or_default();
                match draft.body_kind {
                    BodyKind::Json => Ok(content::pretty_json(&body).unwrap_or(body)),
                    _ => Ok(body),
                }
            }
            EditMode::Headers => draft.headers_text(),
            EditMode::Auth => match draft.auth_value() {
                Some(value) => Ok(value),
                None => return,
            },
            // The environment editor loads its own file; `start_edit` is
            // draft-only.
            EditMode::Environment => return,
        };

        let text = match text {
            Ok(text) => text,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };

        self.method_menu.close();
        self.body_menu.close();
        self.auth_menu.close();

        self.edit_input = TextArea::new(text.lines().map(str::to_string).collect());
        self.edit_input.move_cursor(CursorMove::Bottom);
        self.edit_input.move_cursor(CursorMove::End);
        self.edit_mode = mode;
    }

    fn submit_edit(&mut self) {
        // The environment editor works on a file, not on the draft — it has
        // its own validation and write path.
        if self.edit_mode == EditMode::Environment {
            self.submit_environment_edit();
            return;
        }

        let text = self.edit_input.lines().join("\n");
        let mode = self.edit_mode;
        let should_save = self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.relative.is_some());

        let Some(draft) = self.draft.as_mut() else {
            self.cancel_edit();
            return;
        };

        let result = match mode {
            EditMode::None => Ok(()),
            EditMode::Comments => draft.set_comments(&text),
            EditMode::Url => {
                draft.set_url(&text);
                Ok(())
            }
            EditMode::Body => draft.set_body(&text),
            EditMode::Headers => draft.set_headers_from_text(&text),
            EditMode::Auth => draft.set_auth_value(&text),
            // Returned at the top of this function; here for exhaustiveness.
            EditMode::Environment => unreachable!("handled at the top of submit_edit"),
        };

        match result {
            Ok(()) => {
                if should_save && let Err(error) = self.save_draft() {
                    self.error = Some(error.to_string());
                    return;
                }

                self.error = None;
                self.cancel_edit();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    /// Validates the modal text as a dotenv file and writes it back. The file
    /// is only touched when it parses, and the active environment's variables
    /// are reloaded so the next send sees the new values.
    fn submit_environment_edit(&mut self) {
        let text = self.edit_input.lines().join("\n");

        if let Err(error) = env_file::parse(&text) {
            self.error = Some(error.to_string());
            return; // stays in the editor, the footer shows why
        }

        let Some(name) = self.selected_environment_name() else {
            self.cancel_edit();
            return;
        };

        let content = if text.is_empty() {
            String::new()
        } else {
            format!("{text}\n")
        };

        if let Err(error) = store::overwrite_environment(&self.root, &name, &content) {
            self.error = Some(error.to_string());
            return;
        }

        self.error = None;
        self.cancel_edit();

        if self.active_env.as_deref() == Some(name.as_str()) {
            match store::read_environment(&self.root, &name) {
                Ok(variables) => self.env_vars = variables,
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }

    fn save_current_request(&mut self) {
        match self.save_draft() {
            Ok(()) => self.error = None,
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn save_draft(&self) -> Result<(), AppError> {
        let draft = self
            .draft
            .as_ref()
            .ok_or_else(|| AppError::RequestNotFound("no request loaded".to_string()))?;
        let relative = draft
            .relative
            .as_deref()
            .ok_or_else(|| AppError::RequestNotFound("no request loaded".to_string()))?;
        let content = draft.source()?;

        store::overwrite(&self.root, relative, &content)?;

        Ok(())
    }

    fn cancel_edit(&mut self) {
        self.edit_mode = EditMode::None;
        self.edit_input = TextArea::default();
    }

    fn send_request(&mut self) {
        if matches!(self.send, SendState::Sending) {
            return;
        }
        let Some(draft) = &self.draft else {
            self.send = SendState::Failed("no request loaded".to_string());
            return;
        };
        if draft.relative.is_none() {
            self.send = SendState::Failed("no request loaded".to_string());
            return;
        }
        match draft.to_request(&self.env_vars) {
            Ok(request) => {
                self.pending = Some(request);
                self.error = None;
                self.send = SendState::Sending;
            }
            Err(error) => {
                let mut message = error.to_string();
                if matches!(error, AppError::UnknownVariable(_)) {
                    // Say why the variable is unknown: no environment at all,
                    // or one that simply lacks the name.
                    match &self.active_env {
                        Some(name) => message.push_str(&format!(" (environment: {name})")),
                        None => message.push_str(" (no environment selected)"),
                    }
                }
                self.send = SendState::Failed(message);
            }
        }
    }

    fn spawn_pending(&mut self) {
        let Some(request) = self.pending.take() else {
            return;
        };
        let sender = self.response_tx.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let result = http::execute(&client, &request).await;
            let _ = sender.send(result);
        });
    }

    fn receive_responses(&mut self) {
        while let Ok(result) = self.response_rx.try_recv() {
            self.apply_response(result);
        }
    }

    fn apply_response(&mut self, result: Result<Response, AppError>) {
        self.send = match result {
            Ok(response) => SendState::Done(response),
            Err(error) => SendState::Failed(error.to_string()),
        };
    }
}

#[cfg(test)]
mod tests;

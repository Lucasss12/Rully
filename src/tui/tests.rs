use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use std::fs;
use std::path::PathBuf;

type TestTerminal = Terminal<TestBackend>;

fn terminal() -> TestTerminal {
    terminal_sized(80, 24)
}

fn terminal_sized(width: u16, height: u16) -> TestTerminal {
    Terminal::new(TestBackend::new(width, height)).unwrap()
}

fn draw(app: &mut App, terminal: &mut TestTerminal) {
    terminal.draw(|frame| app.draw(frame)).unwrap();
}

fn press(app: &mut App, code: KeyCode) {
    press_with(app, code, KeyModifiers::NONE);
}

fn press_with(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_key_event(KeyEvent::new(code, modifiers));
}

fn save(app: &mut App) {
    press_with(app, KeyCode::Char('s'), KeyModifiers::CONTROL);
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        press(app, KeyCode::Char(character));
    }
}

fn lines(terminal: &TestTerminal) -> Vec<String> {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect()
}

fn screen(terminal: &TestTerminal) -> String {
    lines(terminal).join("\n")
}

fn border_color(terminal: &TestTerminal, x: u16, y: u16) -> Color {
    terminal.backend().buffer().cell((x, y)).unwrap().fg
}

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        Self::with(
            name,
            &["users"],
            &[(
                "users/list.http",
                "GET https://api.example.com/users\nAccept: application/json\n",
            )],
        )
    }

    fn with(name: &str, collections: &[&str], requests: &[(&str, &str)]) -> Self {
        let dir = std::env::temp_dir().join(format!("rully-tui-{name}"));
        let _ = fs::remove_dir_all(&dir);
        let root = dir.join(store::ROOT_DIR);

        store::init(&root).unwrap();

        for &collection in collections {
            store::create_collection(&root, collection).unwrap();
        }

        for &(relative, content) in requests {
            store::save(&root, relative, content).unwrap();
        }

        Self { dir }
    }

    fn empty(name: &str) -> Self {
        Self::with(name, &[], &[])
    }

    fn root(&self) -> PathBuf {
        self.dir.join(store::ROOT_DIR)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn draws_the_initial_screen() {
    let fixture = Fixture::with(
        "initial-screen",
        &["auth", "users"],
        &[("users/list.http", "GET https://api.example.com/users\n")],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);

    let text = screen(&terminal);
    assert!(text.contains("Collections"), "{text}");
    assert!(text.contains("auth/"), "{text}");
    assert!(text.contains("users/"), "{text}");
    assert!(text.contains("[1 of 2]"), "{text}");
    assert!(text.contains("Enter URL..."), "{text}");
    assert!(text.contains("No headers"), "{text}");
    assert!(text.contains("No response yet"), "{text}");
    assert!(text.contains("q quit"), "{text}");
}

#[test]
fn navigates_and_wraps() {
    let fixture = Fixture::with("navigation", &["alpha", "beta", "gamma"], &[]);
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    assert!(screen(&terminal).contains("[1 of 3]"));

    press(&mut app, KeyCode::Down);
    draw(&mut app, &mut terminal);
    assert_eq!(app.state.selected(), Some(1));
    assert!(screen(&terminal).contains("[2 of 3]"));

    press(&mut app, KeyCode::Up);
    draw(&mut app, &mut terminal);
    assert_eq!(app.state.selected(), Some(0));

    press(&mut app, KeyCode::Up);
    draw(&mut app, &mut terminal);
    assert_eq!(app.state.selected(), Some(2));
    assert!(screen(&terminal).contains("[3 of 3]"));

    press(&mut app, KeyCode::Down);
    draw(&mut app, &mut terminal);
    assert_eq!(app.state.selected(), Some(0));
}

#[test]
fn opens_collection_and_goes_back() {
    let fixture = Fixture::with(
        "open-collection",
        &["users"],
        &[
            ("users/create.http", "POST https://api.example.com/users\n"),
            ("users/list.http", "GET https://api.example.com/users\n"),
        ],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    assert_eq!(app.location.as_deref(), Some("users"));
    let text = screen(&terminal);
    assert!(text.contains("Collections/users"), "{text}");
    assert!(text.contains("create.http"), "{text}");
    assert!(text.contains("list.http"), "{text}");
    assert!(text.contains("No headers"), "{text}");

    press(&mut app, KeyCode::Esc);
    draw(&mut app, &mut terminal);

    assert!(app.location.is_none());
    let text = screen(&terminal);
    assert!(text.contains("Collections"), "{text}");
    assert!(text.contains("users/"), "{text}");
    assert!(!text.contains("create.http"), "{text}");
}

#[test]
fn loads_request_into_the_bar() {
    let fixture = Fixture::new("load-request");
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    let draft = app.draft.as_ref().expect("the request should be loaded");
    assert_eq!(draft.url, "https://api.example.com/users");
    assert_eq!(METHODS[draft.method_index], "GET");

    let text = screen(&terminal);
    assert!(text.contains("https://api.example.com/users"), "{text}");
    assert!(text.contains("application/json"), "{text}");
    assert!(text.contains("No response yet"), "{text}");
}

#[test]
fn shows_sensitive_headers_in_clear() {
    let fixture = Fixture::with(
        "redact-headers",
        &["users"],
        &[(
            "users/list.http",
            "GET https://api.example.com/users\n\
             Accept: application/json\n\
             Authorization: Bearer secret\n",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    let text = screen(&terminal);
    assert!(text.contains("application/json"), "{text}");
    assert!(text.contains("Bearer secret"), "{text}");
    assert!(!text.contains("[redacted]"), "{text}");
}

#[test]
fn tab_cycles_focus() {
    let fixture = Fixture::with("focus-cycle", &["users"], &[]);
    let mut app = App::new(fixture.root());

    assert_eq!(app.focus, Focus::LeftPanel);

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Focus::RequestBar);

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Focus::Request);

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Focus::Response);

    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Focus::LeftPanel);
}

#[test]
fn opens_the_method_menu_on_m() {
    let fixture = Fixture::with(
        "method-menu-open",
        &["users"],
        &[("users/create.http", "POST https://api.example.com/users\n")],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('m'));
    draw(&mut app, &mut terminal);

    assert!(app.method_menu.open);
    assert_eq!(app.method_menu.index, 1);

    let text = screen(&terminal);
    for method in METHODS {
        assert!(text.contains(method), "{method} missing from:\n{text}");
    }
}

#[test]
fn confirms_a_new_method() {
    let fixture = Fixture::with(
        "method-menu-confirm",
        &["users"],
        &[("users/list.http", "GET https://api.example.com/users\n")],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('m'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    assert!(!app.method_menu.open);

    let draft = app.draft.as_ref().expect("the request should be loaded");
    assert_eq!(METHODS[draft.method_index], "POST");

    let text = screen(&terminal);
    assert!(text.contains("POST"), "{text}");
}

#[test]
fn arrows_switch_request_tabs() {
    let fixture = Fixture::with(
        "request-tabs",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\nContent-Type: text/plain\n\nline one",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);

    draw(&mut app, &mut terminal);
    assert!(
        screen(&terminal).contains("text/plain"),
        "{}",
        screen(&terminal)
    );

    press(&mut app, KeyCode::Right);
    draw(&mut app, &mut terminal);
    let text = screen(&terminal);
    assert!(text.contains("line one"), "{text}");
    assert!(!text.contains("No body"), "{text}");

    press(&mut app, KeyCode::Left);
    draw(&mut app, &mut terminal);
    assert!(
        screen(&terminal).contains("text/plain"),
        "{}",
        screen(&terminal)
    );
}

#[test]
fn shows_empty_collections_message() {
    let fixture = Fixture::empty("empty-collections");
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);

    assert_eq!(app.state.selected(), None);
    assert!(
        screen(&terminal).contains("No collections found"),
        "{}",
        screen(&terminal)
    );

    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.state.selected(), None);
}

#[test]
fn ignores_enter_in_an_empty_collection() {
    let fixture = Fixture::with("empty-collection", &["users"], &[]);
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    assert_eq!(app.location.as_deref(), Some("users"));
    assert!(
        screen(&terminal).contains("No requests found"),
        "{}",
        screen(&terminal)
    );

    press(&mut app, KeyCode::Enter);
    assert!(app.draft.as_ref().is_some_and(|draft| draft.url.is_empty()));
}

#[test]
fn shows_body_on_the_body_tab() {
    let fixture = Fixture::with(
        "request-body",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\"}",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal_sized(100, 40);

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    draw(&mut app, &mut terminal);

    let text = screen(&terminal);
    assert!(text.contains("\"name\": \"John\""), "{text}");
    assert!(!text.contains("No body"), "{text}");
}

#[test]
fn shows_placeholders_for_an_empty_request() {
    let fixture = Fixture::with(
        "request-placeholders",
        &["users"],
        &[("users/list.http", "GET https://api.example.com/users\n")],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);
    assert!(
        screen(&terminal).contains("No headers"),
        "{}",
        screen(&terminal)
    );

    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    draw(&mut app, &mut terminal);
    assert!(
        screen(&terminal).contains("No body"),
        "{}",
        screen(&terminal)
    );
}

#[test]
fn reports_an_invalid_request_file() {
    let fixture = Fixture::with(
        "invalid-request",
        &["users"],
        &[("users/bad.http", "GET\n")],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    assert!(app.error.is_some());
    assert!(app.draft.as_ref().is_some_and(|draft| draft.url.is_empty()));
    assert!(
        screen(&terminal).contains("Invalid request file"),
        "{}",
        screen(&terminal)
    );
}

#[test]
fn focused_panel_has_a_cyan_border() {
    let fixture = Fixture::with("focused-border", &["users"], &[]);
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    assert_eq!(border_color(&terminal, 0, 0), Color::Cyan);

    press(&mut app, KeyCode::Tab);
    draw(&mut app, &mut terminal);

    assert_ne!(border_color(&terminal, 0, 0), Color::Cyan);
    assert_eq!(border_color(&terminal, 30, 0), Color::Cyan);
}

#[test]
fn escape_returns_focus_to_the_tree() {
    let fixture = Fixture::with("escape-focus", &["users"], &[]);
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.focus, Focus::Request);

    press(&mut app, KeyCode::Esc);
    assert_eq!(app.focus, Focus::LeftPanel);
}

#[test]
fn cancels_the_method_menu() {
    let fixture = Fixture::with(
        "method-menu-cancel",
        &["users"],
        &[("users/list.http", "GET https://api.example.com/users\n")],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('m'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Esc);
    draw(&mut app, &mut terminal);

    assert!(!app.method_menu.open);

    let draft = app.draft.as_ref().expect("the request should be loaded");
    assert_eq!(METHODS[draft.method_index], "GET");
    assert!(
        !screen(&terminal).contains("PATCH"),
        "{}",
        screen(&terminal)
    );
}

#[test]
fn creates_a_draft_when_none_is_loaded() {
    let fixture = Fixture::with("method-menu-draft", &["users"], &[]);
    let mut app = App::new(fixture.root());
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('m'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    let draft = app.draft.as_ref().expect("the menu should create a draft");
    assert_eq!(METHODS[draft.method_index], "POST");
    assert!(draft.url.is_empty());
    assert!(screen(&terminal).contains("POST"), "{}", screen(&terminal));
}

#[test]
fn opens_the_body_menu_on_b() {
    let fixture = Fixture::with(
        "body-menu-open",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\"}",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal_sized(100, 40);

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('b'));
    draw(&mut app, &mut terminal);

    assert!(app.body_menu.open);
    assert_eq!(app.body_menu.index, 1);
    assert!(!app.method_menu.open);

    let text = screen(&terminal);
    for kind in BODY_KINDS {
        assert!(
            text.contains(kind.label()),
            "{kind:?} missing from:\n{text}"
        );
    }
}

#[test]
fn confirms_a_new_body_type() {
    let fixture = Fixture::with(
        "body-menu-confirm",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\"}",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal_sized(100, 40);

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('b'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    assert!(!app.body_menu.open);

    let draft = app.draft.as_ref().expect("the request should be loaded");
    assert_eq!(draft.body_kind, BodyKind::Form);
    assert_eq!(draft.body.as_deref(), Some(r#"{"name":"John"}"#));
    assert_eq!(
        draft
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/x-www-form-urlencoded")
    );

    assert!(screen(&terminal).contains("Request · form"));
}

#[test]
fn choosing_none_hides_the_body_without_losing_it() {
    let fixture = Fixture::with(
        "body-menu-none",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\"}",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal_sized(100, 40);

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('b'));
    press(&mut app, KeyCode::Up);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    let draft = app.draft.as_ref().expect("the request should be loaded");
    assert_eq!(draft.body_kind, BodyKind::None);
    assert_eq!(draft.body.as_deref(), Some(r#"{"name":"John"}"#));
    assert_eq!(
        draft
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    assert!(screen(&terminal).contains("Request · none"));

    press(&mut app, KeyCode::Right);
    draw(&mut app, &mut terminal);
    assert!(screen(&terminal).contains("No body"));

    press(&mut app, KeyCode::Left);
    press(&mut app, KeyCode::Char('b'));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    draw(&mut app, &mut terminal);

    let draft = app.draft.as_ref().expect("the request should be loaded");
    assert_eq!(draft.body_kind, BodyKind::Json);
    assert_eq!(draft.body.as_deref(), Some(r#"{"name":"John"}"#));
    assert_eq!(
        draft
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    assert!(screen(&terminal).contains("Request · json"));

    press(&mut app, KeyCode::Right);
    draw(&mut app, &mut terminal);
    assert!(screen(&terminal).contains("\"name\": \"John\""));
}

#[test]
fn shows_form_pairs_on_the_body_tab() {
    let fixture = Fixture::with(
        "form-body",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/x-www-form-urlencoded\n\n\
             name=John&age=30",
        )],
    );
    let mut app = App::new(fixture.root());
    let mut terminal = terminal_sized(100, 40);

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    draw(&mut app, &mut terminal);

    let text = screen(&terminal);
    assert!(text.contains("Request · form"), "{text}");
    assert!(text.contains("name = John"), "{text}");
    assert!(text.contains("age = 30"), "{text}");
}

#[test]
fn ignores_b_without_a_request() {
    let fixture = Fixture::with("body-menu-no-draft", &["users"], &[]);
    let mut app = App::new(fixture.root());
    app.draft = None;
    let mut terminal = terminal();

    draw(&mut app, &mut terminal);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('b'));

    assert!(!app.body_menu.open);
    assert!(app.draft.is_none());
}

#[test]
fn edits_the_url_and_saves_with_ctrl_s() {
    let fixture = Fixture::new("edit-url");
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_mode, EditMode::Url);

    type_text(&mut app, "/search?q=rule");

    assert_eq!(app.edit_mode, EditMode::Url);
    assert_eq!(
        app.draft.as_ref().unwrap().url,
        "https://api.example.com/users"
    );

    press(&mut app, KeyCode::Enter);
    assert_eq!(app.edit_mode, EditMode::Url);
    assert_eq!(app.edit_input.lines().len(), 1);

    save(&mut app);

    assert_eq!(app.edit_mode, EditMode::None);
    assert_eq!(
        app.draft.as_ref().unwrap().url,
        "https://api.example.com/users/search?q=rule"
    );
}

#[test]
fn escape_cancels_url_editing() {
    let fixture = Fixture::new("cancel-url-edit");
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('e'));
    type_text(&mut app, "/discarded");
    press(&mut app, KeyCode::Esc);

    assert_eq!(app.edit_mode, EditMode::None);
    assert_eq!(
        app.draft.as_ref().unwrap().url,
        "https://api.example.com/users"
    );
}

#[test]
fn edits_the_body_with_newlines() {
    let fixture = Fixture::with(
        "edit-body",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: text/plain\n\n\
             line one",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_mode, EditMode::Body);

    press(&mut app, KeyCode::Enter);
    type_text(&mut app, "second line");
    save(&mut app);

    let draft = app.draft.as_ref().unwrap();
    assert_eq!(draft.body.as_deref(), Some("line one\nsecond line"));
    assert_eq!(draft.body_kind, BodyKind::Text);
}

#[test]
fn seeds_the_json_body_pretty_printed() {
    let body = r#"{"name":"John","age":30}"#;
    let fixture = Fixture::with(
        "edit-json-pretty",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\",\"age\":30}",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    let expected = content::pretty_json(body).unwrap();
    assert_eq!(app.edit_input.lines().join("\n"), expected);
}

#[test]
fn saves_edited_json_as_written() {
    let fixture = Fixture::with(
        "edit-json-as-written",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\"}",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    let edited = "{\n    \"name\": \"Jane\"\n}";
    app.edit_input = TextArea::new(edited.lines().map(str::to_string).collect());
    save(&mut app);

    assert_eq!(app.edit_mode, EditMode::None);
    assert_eq!(app.draft.as_ref().unwrap().body.as_deref(), Some(edited));
}

#[test]
fn rejects_invalid_json_body() {
    let fixture = Fixture::with(
        "edit-json-invalid",
        &["users"],
        &[(
            "users/create.http",
            "POST https://api.example.com/users\n\
             Content-Type: application/json\n\n\
             {\"name\":\"John\"}",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    app.edit_input = TextArea::new(vec!["{invalid}".to_string()]);
    save(&mut app);

    assert_eq!(app.edit_mode, EditMode::Body);
    assert!(app.error.is_some());
    assert_eq!(
        app.draft.as_ref().unwrap().body.as_deref(),
        Some("{\"name\":\"John\"}")
    );
}

#[test]
fn edits_headers_as_text() {
    let fixture = Fixture::new("edit-headers");
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_mode, EditMode::Headers);

    app.edit_input = TextArea::new(vec![
        "Accept: text/plain".to_string(),
        "X-Test: one".to_string(),
        "X-Test: two".to_string(),
    ]);
    save(&mut app);

    let draft = app.draft.as_ref().unwrap();
    assert_eq!(
        draft
            .headers
            .get("accept")
            .and_then(|value| value.to_str().ok()),
        Some("text/plain")
    );
    assert_eq!(draft.headers.get_all("x-test").iter().count(), 2);
}

#[test]
fn keeps_editing_headers_after_an_invalid_line() {
    let fixture = Fixture::new("invalid-header-edit");
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Char('e'));

    let before = app.draft.as_ref().unwrap().headers.clone();
    app.edit_input = TextArea::new(vec!["missing separator".to_string()]);
    save(&mut app);

    assert_eq!(app.edit_mode, EditMode::Headers);
    assert!(app.error.is_some());
    assert_eq!(app.draft.as_ref().unwrap().headers, before);
}

#[test]
fn edits_bearer_auth() {
    let fixture = Fixture::with(
        "edit-bearer",
        &["users"],
        &[(
            "users/list.http",
            "GET https://api.example.com/users\nAuthorization: Bearer old-token\n",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_mode, EditMode::Auth);
    assert_eq!(app.edit_input.lines().join("\n"), "old-token");

    app.edit_input = TextArea::new(vec!["new-token".to_string()]);
    save(&mut app);

    assert_eq!(
        app.draft
            .as_ref()
            .unwrap()
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer new-token")
    );
}

#[test]
fn edits_basic_auth_as_user_and_password() {
    let fixture = Fixture::with(
        "edit-basic",
        &["users"],
        &[(
            "users/list.http",
            "GET https://api.example.com/users\nAuthorization: Basic dXNlcjpwYXNz\n",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_input.lines().join("\n"), "user:pass");

    app.edit_input = TextArea::new(vec!["new:secret".to_string()]);
    save(&mut app);

    assert_eq!(
        app.draft
            .as_ref()
            .unwrap()
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Basic bmV3OnNlY3JldA==")
    );
}

#[test]
fn rejects_basic_auth_without_a_colon() {
    let fixture = Fixture::with(
        "invalid-basic",
        &["users"],
        &[(
            "users/list.http",
            "GET https://api.example.com/users\nAuthorization: Basic dXNlcjpwYXNz\n",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    app.edit_input = TextArea::new(vec!["missing-colon".to_string()]);
    save(&mut app);

    assert_eq!(app.edit_mode, EditMode::Auth);
    assert!(app.error.is_some());
    assert_eq!(
        app.draft
            .as_ref()
            .unwrap()
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Basic dXNlcjpwYXNz")
    );
}

#[test]
fn edits_api_key_auth() {
    let fixture = Fixture::with(
        "edit-api-key",
        &["users"],
        &[(
            "users/list.http",
            "GET https://api.example.com/users\nX-API-Key: old-key\n",
        )],
    );
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_input.lines().join("\n"), "old-key");

    app.edit_input = TextArea::new(vec!["new-key".to_string()]);
    save(&mut app);

    assert_eq!(
        app.draft
            .as_ref()
            .unwrap()
            .headers
            .get("x-api-key")
            .and_then(|value| value.to_str().ok()),
        Some("new-key")
    );
}

#[test]
fn does_not_edit_auth_when_none_is_selected() {
    let fixture = Fixture::new("edit-no-auth");
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Char('e'));

    assert_eq!(app.edit_mode, EditMode::None);
}

#[test]
fn q_exits() {
    let fixture = Fixture::empty("quit");
    let mut app = App::new(fixture.root());

    press(&mut app, KeyCode::Char('q'));

    assert!(app.exit);
}

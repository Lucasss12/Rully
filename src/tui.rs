use std::{
    io,
    path::{Path, PathBuf},
};

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{ListState, Paragraph},
};

use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use rully_core::errors::AppError;
use rully_core::store;
use rully_core::{http_file, request::Request};

pub fn run(root: PathBuf) -> Result<(), AppError> {
    ratatui::run(|terminal| App::new(root).run(terminal)).map_err(AppError::Terminal)
}

mod keys;
use keys::Action;

mod left_panel;
use left_panel::LeftPanel;

mod view;

mod methods_menu;

mod request_bar;

mod request_panel;
use request_panel::RequestTab;

mod response_panel;

mod content;
use content::{BODY_KINDS, BodyKind};

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

    // Focus and the menus that float over the interface.
    focus: Focus,
    method_menu: Menu, // the method picker: open flag + highlighted entry
    body_menu: Menu,   // the body type picker, anchored under the request panel

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
}

/// The request being viewed: a decoded `.http` file held in memory. It is a
/// draft because the TUI will eventually edit it before saving.
#[derive(Debug)]
struct Draft {
    method_index: usize,
    url: String,
    headers: HeaderMap,
    body: Option<String>,
    body_kind: BodyKind,
}

impl Draft {
    fn empty() -> Self {
        Self {
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

const METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

impl App {
    /// Runs the draw/read-keys loop until `quit` is dispatched.
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn new(root: PathBuf) -> Self {
        let mut app = Self {
            root,
            items: Vec::new(),
            error: None,
            state: ListState::default(),
            location: None,
            draft: None,
            request_tab: RequestTab::Headers,
            focus: Focus::LeftPanel,
            method_menu: Menu::new(),
            body_menu: Menu::new(),
            exit: false,
        };
        app.refresh();
        app
    }

    /// Reloads `items` for the current level: the collections at the root, or
    /// the requests of the open collection.
    fn refresh(&mut self) {
        let result = match &self.location {
            None => store::list_collections(&self.root),
            Some(name) => store::list_requests(&self.root, name),
        };

        match result {
            Ok(names) => {
                self.state
                    .select(if names.is_empty() { None } else { Some(0) });
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

    fn draw(&mut self, frame: &mut Frame) {
        let title = view::bread_crumb(self.location.as_deref());
        let empty = view::empty_message(self.location.as_deref());

        let [main, help] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
        let [tree, detail] =
            Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)]).areas(main);
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

        let draft = self.draft.as_ref();
        request_bar::render(frame, bar, draft, self.focus == Focus::RequestBar);
        request_panel::render(
            frame,
            request,
            draft,
            self.request_tab,
            self.focus == Focus::Request,
        );
        response_panel::render(frame, response, self.focus == Focus::Response);

        let menu_open = self.method_menu.open || self.body_menu.open;

        frame.render_widget(
            Paragraph::new(keys::help_line(keys::current_binding(
                self.focus,
                self.location.as_deref(),
                menu_open,
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
    }

    fn handle_events(&mut self) -> io::Result<()> {
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
        let Some(binding) = keys::current_binding(
            self.focus,
            self.location.as_deref(),
            self.method_menu.open || self.body_menu.open,
        )
        .iter()
        .find(|b| b.key == key_event.code) else {
            return;
        };

        self.dispatch(binding.action);
    }

    fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit(),
            Action::SelectPrevious => self.select_previous(),
            Action::SelectNext => self.select_next(),
            Action::Activate => self.activate(),
            Action::GoBack => self.go_back(),

            Action::NextFocus => self.next_focus(),
            Action::MenuPrevious => self.menu_previous(),
            Action::MenuNext => self.menu_next(),
            Action::MenuConfirm => self.menu_confirm(),
            Action::MenuCancel => self.menu_cancel(),

            Action::TabPrevious => self.switch_tab(),
            Action::TabNext => self.switch_tab(),
            Action::OpenMethodMenu => self.open_method_menu(),
            Action::OpenBodyMenu => self.open_body_menu(),
            Action::FocusLeftPanel => self.focus = Focus::LeftPanel,
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
    fn read_draft(root: &Path, relative: &str) -> Result<Draft, AppError> {
        let path = store::request_path(root, relative)?;
        let file = http_file::read(&path.to_string_lossy())?;
        let request = Request::try_from(file)?;
        let body_kind = BodyKind::of(&request.headers, request.body.as_deref());
        let method_index = METHODS
            .iter()
            .position(|method| *method == request.method.as_str())
            .unwrap_or(0);

        Ok(Draft {
            method_index,
            url: request.url.to_string(),
            headers: request.headers,
            body: request.body,
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
            Ok(draft) => self.draft = Some(draft),
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
            Focus::Response => Focus::LeftPanel,
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
        self.body_menu.show(index);
    }

    fn menu_previous(&mut self) {
        if self.method_menu.open {
            self.method_menu.previous(METHODS.len());
        } else if self.body_menu.open {
            self.body_menu.previous(BODY_KINDS.len());
        }
    }

    fn menu_next(&mut self) {
        if self.method_menu.open {
            self.method_menu.next(METHODS.len());
        } else if self.body_menu.open {
            self.body_menu.next(BODY_KINDS.len());
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
        }
    }

    fn menu_cancel(&mut self) {
        self.method_menu.close();
        self.body_menu.close();
    }

    fn switch_tab(&mut self) {
        self.request_tab = RequestTab::switch_tabs(self.request_tab);
    }
}

#[cfg(test)]
mod tests;

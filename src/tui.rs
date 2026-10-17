use std::{io, path::PathBuf};

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    widgets::{Block, ListState, Paragraph},
};

use rully_core::errors::AppError;
use rully_core::store;

pub fn run(root: PathBuf) -> Result<(), AppError> {
    ratatui::run(|terminal| App::new(root).run(terminal)).map_err(AppError::Terminal)
}

mod keys;
use keys::Action;

mod left_panel;
use left_panel::LeftPanel;

mod view;

#[derive(Debug)]
pub struct App {
    root: PathBuf,
    items: Vec<String>,
    error: Option<String>,
    exit: bool,
    state: ListState,
    open_collection: Option<String>,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub fn new(root: PathBuf) -> Self {
        let mut app = Self {
            root,
            items: Vec::new(),
            error: None,
            exit: false,
            state: ListState::default(),
            open_collection: None,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        let result = match &self.open_collection {
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
        let title = view::bread_crumb(self.open_collection.as_deref());
        let empty = view::empty_message(self.open_collection.as_deref());

        let [main, bottom] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());
        let [left, right] =
            Layout::horizontal([Constraint::Length(30), Constraint::Fill(1)]).areas(main);

        frame.render_stateful_widget(
            LeftPanel {
                items: &self.items,
                error: self.error.as_deref(),
                title: &title,
                empty,
            },
            left,
            &mut self.state,
        );

        frame.render_widget(
            Paragraph::new("").block(Block::bordered().title("droite")),
            right,
        );
        frame.render_widget(
            Paragraph::new(keys::help_line(keys::current_binding(
                self.open_collection.as_deref(),
            ))),
            bottom,
        );
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

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        let Some(binding) = keys::current_binding(self.open_collection.as_deref())
            .iter()
            .find(|b| b.key == key_event.code)
        else {
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
        }
    }

    fn select_next(&mut self) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        let next = self.state.selected().map_or(0, |i| (i + 1) % len);
        self.state.select(Some(next));
    }

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

    fn selected_entry(&self) -> Option<&str> {
        let index = self.state.selected()?;
        self.items.get(index).map(String::as_str)
    }

    fn activate(&mut self) {
        if self.open_collection.is_some() {
            return;
        }
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if let Some(name) = entry.strip_suffix('/') {
            self.open_collection = Some(name.to_string());
            self.refresh();
        }
    }

    fn go_back(&mut self) {
        if self.open_collection.take().is_some() {
            self.refresh();
        }
    }

    fn quit(&mut self) {
        self.exit = true;
    }
}

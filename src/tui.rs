use std::{io, path::PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::text::Line;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, List, ListState, Paragraph, StatefulWidget, Widget},
};

use rully_core::errors::AppError;
use rully_core::store;

pub fn run(root: PathBuf) -> Result<(), AppError> {
    ratatui::run(|terminal| App::new(root).run(terminal)).map_err(AppError::Terminal)
}

#[derive(Debug)]
pub struct App {
    root: PathBuf,      // chemin du dossier de collection
    items: Vec<String>, // liste de collections
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
        // 1) UN layout, deux contraintes -> deux Rect
        let title = self.bread_crumb();
        let empty = self.empty_message();
        let [gauche, droite] = Layout::horizontal([
            Constraint::Length(30), // zone gauche : 30 colonnes
            Constraint::Fill(1),    // zone droite : le reste
        ])
        .areas(frame.area());

        // 2) un widget par Rect
        frame.render_stateful_widget(
            LeftPanel {
                items: &self.items,
                error: self.error.as_deref(),
                title: &title,
                empty: &empty,
            },
            gauche,
            &mut self.state,
        );
        frame.render_widget(Paragraph::new("colonne droite"), droite);
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
        match key_event.code {
            KeyCode::Char('q') => self.quit(),
            KeyCode::Up => self.select_previous(),
            KeyCode::Down => self.select_next(),
            KeyCode::Enter => self.activate(),
            KeyCode::Esc => self.go_back(),
            _ => {}
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
            return; // déjà dans une collection : pas de sous-niveau
        }
        let Some(entry) = self.selected_entry() else {
            return;
        };
        if let Some(name) = entry.strip_suffix('/') {
            self.open_collection = Some(name.to_string());
            self.refresh();
        }
    }

    fn bread_crumb(&self) -> String {
        match &self.open_collection {
            None => "Collections".to_string(),
            Some(name) => format!("Collections/{name}"),
        }
    }

    fn empty_message(&self) -> &'static str {
        match &self.open_collection {
            None => "No collections found",
            Some(_) => "No requests found",
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

struct LeftPanel<'a> {
    items: &'a [String],
    error: Option<&'a str>,
    title: &'a str,
    empty: &'a str,
}

impl StatefulWidget for LeftPanel<'_> {
    type State = ListState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        if let Some(error) = self.error {
            Paragraph::new(error) // erreur affichée à gauche
                .block(Block::bordered().title(self.title))
                .render(area, buf);
            return;
        }
        if self.items.is_empty() {
            Paragraph::new(self.empty)
                .block(Block::bordered().title(self.title))
                .render(area, buf);
            return;
        }

        let position = state.selected().map_or(0, |index| index + 1);
        let label = format!("[{position} of {}]", self.items.len());

        let block = Block::bordered()
            .title(self.title)
            .title_bottom(Line::from(label).right_aligned());

        let items = self.items.iter().map(String::as_str);
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().reversed());
        StatefulWidget::render(list, area, buf, state);
    }
}

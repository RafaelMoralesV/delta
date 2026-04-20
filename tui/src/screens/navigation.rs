use crate::{App, AppState, Message, screens::project_view::ProjectModel};
use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::{fs, path::PathBuf};

pub struct NavModel {
    pub current_dir: PathBuf,
    pub items: Vec<String>,
    pub list_state: ListState,
}

pub enum NavMsg {
    MoveUp,
    MoveDown,
    Select,
    MoveInto,
    ToConfig,
}

impl From<NavMsg> for Message {
    fn from(value: NavMsg) -> Self {
        Message::Navigation(value)
    }
}

pub fn handle_key_event(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Up => Some(NavMsg::MoveUp.into()),
        KeyCode::Down => Some(NavMsg::MoveDown.into()),
        KeyCode::Enter => Some(NavMsg::Select.into()),
        KeyCode::Char('c') => Some(NavMsg::ToConfig.into()),
        KeyCode::Char(' ') => Some(NavMsg::MoveInto.into()),
        KeyCode::Char('k') => Some(NavMsg::MoveUp.into()),
        KeyCode::Char('j') => Some(NavMsg::MoveDown.into()),
        KeyCode::Esc => Some(Message::Quit),
        _ => None,
    }
}

impl NavModel {
    pub fn new(path: PathBuf) -> Self {
        let mut model = Self {
            current_dir: path,
            items: vec![],
            list_state: ListState::default(),
        };
        model.load_directory();
        model
    }

    pub fn load_directory(&mut self) {
        self.items.clear();
        self.items.push(".. (Up)".to_string());

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            dirs.sort();
            self.items.extend(dirs);
        }
        self.list_state.select(Some(0));
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

pub fn view(model: &mut NavModel, f: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = model
        .items
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" Navigating: {} ", model.current_dir.display()))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut model.list_state);
}

pub fn update(app: &mut App, msg: NavMsg) {
    match msg {
        NavMsg::ToConfig => app.state = AppState::Configuring,
        NavMsg::MoveUp => app.navigation_state.previous(),
        NavMsg::MoveDown => app.navigation_state.next(),
        NavMsg::MoveInto => {
            if let Some(i) = app.navigation_state.list_state.selected() {
                let selected = app.navigation_state.items[i].clone();

                if selected == ".. (Up)" {
                    app.navigation_state.current_dir.pop();
                    app.navigation_state.load_directory();
                } else {
                    let new_path = app.navigation_state.current_dir.join(&selected);
                    if new_path.is_dir() {
                        app.navigation_state.current_dir = new_path;
                        app.navigation_state.load_directory();
                    }
                }
            }
        }
        NavMsg::Select => {
            if let Some(i) = app.navigation_state.list_state.selected() {
                let selected = app.navigation_state.items[i].clone();

                // El usuario puede seleccionar nuestra meta-carpeta. En ese caso, lo mandamos de vuelta pa arriba como
                // si fuera un MoveInto.
                if selected == ".. (Up)" {
                    app.navigation_state.current_dir.pop();
                    app.navigation_state.load_directory();
                } else {
                    let new_path = app.navigation_state.current_dir.join(&selected);
                    let versions = core::get_current_versions(&new_path, &selected);
                    app.state = AppState::ViewingProject(ProjectModel {
                        name: selected,
                        versions,
                    });
                }
            }
        }
    }
}

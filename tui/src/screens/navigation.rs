use crate::{App, AppState, Message, config::AppConfig, screens::project_view::ProjectModel};
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode, KeyModifiers},
    layout::Rect,
    style::Style,
    widgets::{List, ListItem, ListState},
};
use std::{fs, path::PathBuf};

pub struct NavModel {
    pub current_dir: PathBuf,
    pub items: Vec<String>,          // Full list
    pub filtered_items: Vec<String>, // Visible list based on search
    pub list_state: ListState,
    pub search_query: String, // The current input
    pub is_searching: bool,   // Mode toggle
}

pub enum NavMsg {
    MoveUp,
    MoveDown,
    Select,
    MoveInto,
    MoveOut,
    ToConfig,
    StartSearch,
    UpdateSearch(String),
    EndSearch(bool), // true to apply, false to cancel
}

impl From<NavMsg> for Message {
    fn from(value: NavMsg) -> Self {
        Message::Navigation(value)
    }
}

pub fn handle_key_event(key: event::KeyEvent, state: &NavModel) -> Option<Message> {
    let is_searching = state.is_searching;

    // 1. Handle Global Mode Toggles
    if !is_searching {
        if (key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('/')
        {
            return Some(NavMsg::StartSearch.into());
        }
    }

    // 2. Handle Search Mode Input
    if is_searching {
        return match key.code {
            KeyCode::Enter => Some(NavMsg::EndSearch(true).into()),
            KeyCode::Esc => Some(NavMsg::EndSearch(false).into()),
            KeyCode::Backspace => Some(NavMsg::UpdateSearch("BACKSPACE".to_string()).into()),
            KeyCode::Char(c) => Some(NavMsg::UpdateSearch(c.to_string()).into()),
            _ => None,
        };
    }

    let message = match key.code {
        KeyCode::Up => NavMsg::MoveUp,
        KeyCode::Down => NavMsg::MoveDown,
        KeyCode::Enter => NavMsg::Select,
        KeyCode::Backspace => NavMsg::MoveOut,
        KeyCode::Char('c') => NavMsg::ToConfig,
        KeyCode::Char(' ') => NavMsg::MoveInto,
        KeyCode::Char('k') => NavMsg::MoveUp,
        KeyCode::Char('j') => NavMsg::MoveDown,
        KeyCode::Esc => return Some(Message::Quit),
        _ => return None,
    };

    Some(message.into())
}

impl NavModel {
    pub fn new(path: PathBuf) -> Self {
        let mut model = Self {
            current_dir: path,
            items: vec![],
            list_state: ListState::default(),
            filtered_items: vec![],
            is_searching: false,
            search_query: String::new(),
        };
        model.load_directory();
        model.apply_filter();
        model
    }

    pub fn load_directory(&mut self) {
        self.items.clear();
        self.items.push(".. (Subir)".to_string());

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            dirs.sort();
            self.items.extend(dirs.clone());
        }
        self.list_state.select(Some(0));
        self.search_query = String::new();
        self.apply_filter();
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_items.len() - 1 {
                    self.filtered_items.len() - 1
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
                    i
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_items = self.items.clone();
        } else {
            let query = self.search_query.to_lowercase();
            let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
            self.filtered_items = self
                .items
                .iter()
                .filter(|item| matcher.fuzzy_match(item, &query).is_some())
                .cloned()
                .collect();
        }
        // Reset selection to the top after filtering
        self.list_state.select(Some(0));
    }
}

pub fn view(model: &mut NavModel, f: &mut Frame, area: Rect, cfg: &AppConfig) {
    let p = &cfg.palette;
    let title = format!("Navegando: {}", model.current_dir.to_string_lossy());

    // Change footer text based on mode
    let help_text = if model.is_searching {
        format!(
            "SEARCHING: {} | Enter: Confirm | Esc: Cancel",
            model.search_query
        )
    } else {
        "↑/↓: Nav | /: Search | Enter: Open".to_string()
    };

    crate::components::page::draw_page(f, area, &title, &help_text, cfg, |f, chunk| {
        let items: Vec<ListItem> = model
            .filtered_items
            .iter()
            .map(|i| ListItem::new(i.as_str()).style(Style::default().fg(p.get_color(&p.base05))))
            .collect();

        let list = List::new(items).highlight_symbol(">> ").highlight_style(
            Style::default()
                .bg(p.get_color(&p.base02)) // Selection background
                .fg(p.get_color(&p.base0B)) // Selected text/symbol color (Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

        f.render_stateful_widget(list, chunk, &mut model.list_state);
    });
}

pub fn update(app: &mut App, msg: NavMsg) {
    match msg {
        NavMsg::ToConfig => app.state = AppState::Configuring,
        NavMsg::MoveUp => app.navigation_state.previous(),
        NavMsg::MoveDown => app.navigation_state.next(),
        NavMsg::MoveOut => {
            app.navigation_state.current_dir.pop();
            app.navigation_state.load_directory();
        }
        NavMsg::MoveInto => {
            if let Some(i) = app.navigation_state.list_state.selected() {
                let selected = app.navigation_state.filtered_items[i].clone();

                if selected == ".. (Subir)" {
                    return update(app, NavMsg::MoveOut);
                }
                let new_path = app.navigation_state.current_dir.join(&selected);
                if new_path.is_dir() {
                    app.navigation_state.current_dir = new_path;
                    app.navigation_state.load_directory();
                }
            }
        }
        NavMsg::Select => {
            if let Some(i) = app.navigation_state.list_state.selected() {
                let selected = app.navigation_state.filtered_items[i].clone();

                // El usuario puede seleccionar nuestra meta-carpeta. En ese caso, lo mandamos de vuelta pa arriba como
                // si fuera un MoveInto.
                if selected == ".. (Subir)" {
                    return update(app, NavMsg::MoveOut);
                }
                let new_path = app.navigation_state.current_dir.join(&selected);
                let versions = core::get_current_versions(&new_path, &selected);
                app.state = AppState::ViewingProject(ProjectModel::new(selected, versions));
            }
        }
        NavMsg::StartSearch => {
            app.navigation_state.is_searching = true;
            app.navigation_state.search_query.clear();
        }
        NavMsg::UpdateSearch(input) => {
            if input == "BACKSPACE" {
                app.navigation_state.search_query.pop();
            } else {
                app.navigation_state.search_query.push_str(&input);
            }
            app.navigation_state.apply_filter();
        }
        NavMsg::EndSearch(apply) => {
            app.navigation_state.is_searching = false;
            if !apply {
                app.navigation_state.search_query.clear();
                app.navigation_state.apply_filter();
            }
        }
    }
}

use crate::{
    App, AppFocus, Message,
    components::layout::{AppLayout, Panel},
    config::AppConfig,
    screens::project_view::ProjectModel,
};
use core::{
    FileHandler, NavigationError, NavigationService, OsFileSystem,
    navigation::{DirectoryDisplayInfo, ProjectType},
};
use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode, KeyModifiers},
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
};
use std::{collections::HashMap, path::PathBuf};

pub struct NavModel {
    pub current_dir: PathBuf,
    pub items: Vec<String>,
    pub directory_info: std::collections::HashMap<String, DirectoryDisplayInfo>,
    pub filtered_items: Vec<(String, Vec<usize>)>,
    pub list_state: ListState,
    pub search_query: String,
    pub is_searching: bool,
    pub error: Option<String>,
    pub fs: OsFileSystem,
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
    EndSearch(bool),
    ClearError,
}

impl From<NavMsg> for Message {
    fn from(value: NavMsg) -> Self {
        Message::Navigation(value)
    }
}

pub fn handle_key_event(key: event::KeyEvent, state: &NavModel) -> Option<Message> {
    if state.error.is_some() {
        return Some(NavMsg::ClearError.into());
    }

    let is_searching = state.is_searching;

    if !is_searching
        && ((key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL))
            || key.code == KeyCode::Char('/'))
    {
        return Some(NavMsg::StartSearch.into());
    }

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
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => return Some(Message::Quit),
        _ => return None,
    };

    Some(message.into())
}

impl NavModel {
    pub fn new(fs: &impl FileHandler, path: PathBuf) -> Self {
        let mut model = Self {
            current_dir: path,
            items: vec![],
            list_state: ListState::default(),
            filtered_items: vec![],
            is_searching: false,
            search_query: String::new(),
            error: None,
            directory_info: HashMap::new(),
            fs: OsFileSystem,
        };

        model.load_directory(fs);
        model
    }

    pub fn load_directory(&mut self, fs: &impl FileHandler) {
        self.items.clear();
        self.directory_info.clear();
        self.items.push(".. (Subir)".to_string());

        match NavigationService::list_directories(fs, &self.current_dir) {
            Ok(dirs) => {
                for dir in &dirs {
                    let full_path = self.current_dir.join(dir);
                    let info = NavigationService::get_directory_display_info(fs, &full_path);
                    self.directory_info.insert(dir.clone(), info);
                }
                self.items.extend(dirs);
            }
            Err(_) => {}
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
        let matches = NavigationService::fuzzy_filter(&self.items, &self.search_query, false);
        self.filtered_items = matches
            .into_iter()
            .map(|m| (m.text, m.match_indices))
            .collect();
        self.list_state.select(Some(0));
    }

    pub fn move_to_parent(&mut self, fs: &impl FileHandler) -> anyhow::Result<()> {
        if let Some(parent) = NavigationService::parent_directory(&self.current_dir) {
            NavigationService::validate_navigation_path(fs, &parent)?;
            self.current_dir = parent;
            self.load_directory(fs);
            Ok(())
        } else {
            Err(NavigationError::AtRootDirectory.into())
        }
    }

    pub fn move_into_directory(
        &mut self,
        fs: &impl FileHandler,
        dir_name: &str,
    ) -> anyhow::Result<()> {
        let sanitized = NavigationService::sanitize_path_component(dir_name)?;
        let new_path = NavigationService::join_paths(&self.current_dir, &sanitized)?;
        NavigationService::validate_navigation_path(fs, &new_path)?;
        self.current_dir = new_path;
        self.load_directory(fs);
        Ok(())
    }

    pub fn current_path_string(&self) -> String {
        self.current_dir.to_string_lossy().to_string()
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn get_selected_preview(&self) -> Option<(String, DirectoryDisplayInfo, PathBuf)> {
        let i = self.list_state.selected()?;
        let selected = &self.filtered_items[i].0;

        if selected == ".. (Subir)" {
            return None;
        }

        let display_info = self.directory_info.get(selected)?.clone();
        let full_path = self.current_dir.join(selected);

        Some((selected.clone(), display_info, full_path))
    }
}

pub fn view(model: &mut NavModel, f: &mut Frame, area: Rect, cfg: &AppConfig) {
    let help_text = if let Some(error) = &model.error {
        format!("ERROR: {} | Press any key to continue", error)
    } else if model.is_searching {
        format!(
            "BUSCANDO: {} | Enter: Confirmar | Esc: Cancelar",
            model.search_query
        )
    } else {
        "↑/↓: Nav | /: Buscar | c: Config | Espacio: Entrar | Enter: Ver".to_string()
    };

    let layout = AppLayout::new(model.current_path_string(), help_text);

    layout.render(f, area, cfg, |f, inner| {
        let panel = Panel::new().title("Directories");

        panel.render(f, inner, cfg, |f, list_area| {
            if model.filtered_items.is_empty() && !model.items.is_empty() {
                let no_matches = List::new(vec![ListItem::new("No matching directories found")]);
                f.render_widget(no_matches, list_area);
                return;
            }

            let items: Vec<ListItem> = model
                .filtered_items
                .iter()
                .map(|(text, indices)| {
                    let display_info = model.directory_info.get(text);
                    create_directory_list_item(text, indices, display_info, cfg)
                })
                .collect();

            let list = List::new(items).highlight_symbol(">> ").highlight_style(
                Style::default()
                    .bg(cfg.palette.get_color(&cfg.palette.base02))
                    .add_modifier(ratatui::style::Modifier::BOLD),
            );

            f.render_stateful_widget(list, list_area, &mut model.list_state);
        });
    });
}

fn create_directory_list_item(
    name: &str,
    match_indices: &[usize],
    display_info: Option<&DirectoryDisplayInfo>,
    cfg: &AppConfig,
) -> ListItem<'static> {
    let p = &cfg.palette;
    let mut spans = Vec::new();

    if name != ".. (Subir)" {
        if let Some(info) = display_info {
            let indicator = get_project_indicator(&info.project_type);
            let style = get_project_type_style(&info.project_type, p);
            spans.push(Span::styled(indicator.to_string(), style));
        } else {
            spans.push(Span::raw("  "));
        }
    }

    for (i, c) in name.chars().enumerate() {
        if match_indices.contains(&i) {
            spans.push(Span::styled(
                c.to_string(),
                Style::default().fg(p.get_color(&p.base0A)).add_modifier(
                    ratatui::style::Modifier::BOLD | ratatui::style::Modifier::UNDERLINED,
                ),
            ));
        } else {
            spans.push(Span::raw(c.to_string()));
        }
    }

    if let Some(info) = display_info
        && let Some(versions) = &info.versions
    {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            versions.clone(),
            Style::default()
                .fg(p.get_color(&p.base03))
                .add_modifier(ratatui::style::Modifier::DIM),
        ));
    }

    ListItem::new(Line::from(spans))
}

fn get_project_indicator(project_type: &ProjectType) -> &str {
    match project_type {
        ProjectType::FullStack => "● ",
        ProjectType::Web => "◒ ",
        ProjectType::BusinessLayer => "◓ ",
        ProjectType::GitRepository => "○ ",
        ProjectType::Unknown => "  ",
    }
}

fn get_project_type_style(project_type: &ProjectType, p: &crate::config::Palette) -> Style {
    match project_type {
        ProjectType::FullStack => Style::default()
            .fg(p.get_color(&p.base0B))
            .add_modifier(ratatui::style::Modifier::BOLD),
        ProjectType::Web => Style::default().fg(p.get_color(&p.base0D)),
        ProjectType::BusinessLayer => Style::default().fg(p.get_color(&p.base0E)),
        ProjectType::GitRepository => Style::default()
            .fg(p.get_color(&p.base03))
            .add_modifier(ratatui::style::Modifier::DIM),
        ProjectType::Unknown => Style::default(),
    }
}

pub fn update(app: &mut App, msg: NavMsg) {
    match msg {
        NavMsg::ToConfig => app.current_focused_screen = AppFocus::Configuring,
        NavMsg::MoveUp => app.navigation_state.previous(),
        NavMsg::MoveDown => app.navigation_state.next(),
        NavMsg::MoveOut => {
            if let Err(e) = app.navigation_state.move_to_parent(&app.fs) {
                app.navigation_state.error = Some(format!("Cannot move up: {}", e));
            }
        }
        NavMsg::MoveInto => {
            if let Some(i) = app.navigation_state.list_state.selected() {
                let selected = app.navigation_state.filtered_items[i].0.clone();
                if selected == ".. (Subir)" {
                    return update(app, NavMsg::MoveOut);
                }
                // Use the NavigationService method with proper validation
                if let Err(e) = app.navigation_state.move_into_directory(&app.fs, &selected) {
                    app.navigation_state.error = Some(format!("Cannot enter directory: {}", e));
                }
            }
        }
        NavMsg::Select => {
            if let Some(i) = app.navigation_state.list_state.selected() {
                let selected = app.navigation_state.filtered_items[i].0.clone();
                if selected == ".. (Subir)" {
                    return update(app, NavMsg::MoveOut);
                }
                // Use NavigationService to safely build the path
                match NavigationService::join_paths(&app.navigation_state.current_dir, &selected) {
                    Ok(new_path) => {
                        if NavigationService::validate_navigation_path(&app.fs, &new_path).is_ok() {
                            let versions =
                                core::get_current_versions(&app.fs, &new_path, &selected);

                            app.project_view_model = ProjectModel::new(selected, versions);
                            app.current_focused_screen = AppFocus::ViewingProject;
                        } else {
                            app.navigation_state.error =
                                Some(format!("Cannot access: {}", selected));
                        }
                    }
                    Err(e) => {
                        app.navigation_state.error = Some(format!("Invalid selection: {}", e));
                    }
                }
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
        NavMsg::ClearError => app.navigation_state.clear_error(),
    }
}

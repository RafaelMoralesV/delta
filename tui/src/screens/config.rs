use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use ratatui_textarea::TextArea;

use crate::{App, AppState, Message};

#[derive(PartialEq)]
pub enum ConfigField {
    Username,
    DefaultPath,
}

pub struct ConfigModel<'a> {
    pub username_input: TextArea<'a>,
    pub path_input: TextArea<'a>,
    pub active_field: ConfigField,
}

pub enum ConfigMsg {
    UpdateInput(event::KeyEvent),
    ToggleFocus,
    Save,
    Cancel,
}

impl From<ConfigMsg> for Message {
    fn from(v: ConfigMsg) -> Self {
        Message::Config(v)
    }
}

pub fn handle_key_event(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Enter => Some(ConfigMsg::Save.into()),
        KeyCode::Esc => Some(ConfigMsg::Cancel.into()),
        KeyCode::Tab => Some(ConfigMsg::ToggleFocus.into()),
        _ => Some(ConfigMsg::UpdateInput(key).into()),
    }
}

impl<'a> ConfigModel<'a> {
    pub fn new(username: String, path: PathBuf) -> Self {
        let mut user_area = TextArea::default();
        user_area.insert_str(username);

        let mut path_area = TextArea::default();
        path_area.insert_str(path.to_string_lossy());

        Self {
            username_input: user_area,
            path_input: path_area,
            active_field: ConfigField::Username,
        }
    }

    fn create_block(&self, field: ConfigField, title: &'static str) -> Block<'a> {
        let is_active = self.active_field == field;
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if is_active {
                Color::Yellow
            } else {
                Color::White
            }))
            .title(format!(" {} ", title))
    }
}

pub fn view(model: &ConfigModel, f: &mut Frame, area: Rect) {
    crate::components::page::draw_page(
        f,
        area,
        "Configuration",
        "Tab: Cambiar Campo | Enter: Guardar | Esc: Cancelar",
        |f, chunk| {
            let layout = Layout::vertical([
                Constraint::Length(3), // Usuario
                Constraint::Length(3), // Ruta
                Constraint::Min(0),
            ])
            .split(chunk);

            let mut username_input = model.username_input.clone();
            username_input
                .set_block(model.create_block(ConfigField::Username, "Nombre de Usuario"));

            let mut path_input = model.path_input.clone();
            path_input
                .set_block(model.create_block(ConfigField::DefaultPath, "Ruta Base de Proyectos"));

            f.render_widget(&username_input, layout[0]);
            f.render_widget(&path_input, layout[1]);
        },
    );
}

pub fn update(app: &mut App, msg: ConfigMsg) {
    match msg {
        ConfigMsg::ToggleFocus => {
            app.config_model.active_field = match app.config_model.active_field {
                ConfigField::Username => ConfigField::DefaultPath,
                ConfigField::DefaultPath => ConfigField::Username,
            };
        }
        ConfigMsg::Save => {
            app.config.username = app.config_model.username_input.lines()[0]
                .trim()
                .to_string();
            let path_str = app.config_model.path_input.lines()[0].trim().to_string();
            app.config.default_dir = PathBuf::from(path_str);

            let _ = app.config.save();
            app.state = AppState::Navigating;
        }
        ConfigMsg::Cancel => app.state = AppState::Navigating,
        ConfigMsg::UpdateInput(ev) => {
            match app.config_model.active_field {
                ConfigField::Username => app.config_model.username_input.input(ev),
                ConfigField::DefaultPath => app.config_model.path_input.input(ev),
            };
        }
    }
}

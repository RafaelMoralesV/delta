use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};
use ratatui_textarea::TextArea;

use crate::{
    App, AppState, Message,
    config::{AppConfig, Palette},
};

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

    fn create_block(&self, field: ConfigField, title: &'static str, p: &Palette) -> Block<'a> {
        let is_active = self.active_field == field;

        let border_color = if is_active {
            p.get_color(&p.base0A) // Use Yellow/Accent for active field
        } else {
            p.get_color(&p.base03) // Use "Invisibles/Comments" grey for inactive
        };

        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {} ", title))
            // Optional: subtle background for the active field
            .style(Style::default().bg(if is_active {
                p.get_color(&p.base01)
            } else {
                p.get_color(&p.base00)
            }))
    }
}

pub fn view(model: &ConfigModel, f: &mut Frame, area: Rect, cfg: &AppConfig) {
    let p = &cfg.palette;

    crate::components::page::draw_page(
        f,
        area,
        "Configuration",
        "Tab: Cambiar Campo | Enter: Guardar | Esc: Cancelar",
        cfg,
        |f, chunk| {
            let layout = Layout::vertical([
                Constraint::Length(3), // Usuario
                Constraint::Length(3), // Ruta
                Constraint::Min(0),
            ])
            .split(chunk);

            // Style Username Input
            let mut username_input = model.username_input.clone();
            username_input.set_block(model.create_block(
                ConfigField::Username,
                "Nombre de Usuario",
                p,
            ));
            username_input.set_cursor_style(
                Style::default()
                    .bg(p.get_color(&p.base05))
                    .fg(p.get_color(&p.base00)),
            );
            username_input.set_style(Style::default().fg(p.get_color(&p.base05)));

            // Style Path Input
            let mut path_input = model.path_input.clone();
            path_input.set_block(model.create_block(
                ConfigField::DefaultPath,
                "Ruta Base de Proyectos",
                p,
            ));
            path_input.set_cursor_style(
                Style::default()
                    .bg(p.get_color(&p.base05))
                    .fg(p.get_color(&p.base00)),
            );
            path_input.set_style(Style::default().fg(p.get_color(&p.base05)));

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

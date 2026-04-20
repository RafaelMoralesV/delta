use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::TextArea;

use crate::{AppState, Message, config::AppConfig};

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
    let layout = Layout::vertical([
        Constraint::Length(1), // Help
        Constraint::Length(3), // Username
        Constraint::Length(3), // Path
        Constraint::Min(0),
    ])
    .split(area);

    f.render_widget(
        Paragraph::new("Tab: Switch Fields | Enter: Save | Esc: Cancel")
            .alignment(Alignment::Center),
        layout[0],
    );

    // We clone the textareas (cheaply) or just modify them temporarily for render
    let mut username_input = model.username_input.clone();
    username_input.set_block(model.create_block(ConfigField::Username, "User Name"));

    let mut path_input = model.path_input.clone();
    path_input.set_block(model.create_block(ConfigField::DefaultPath, "Default Project Path"));

    f.render_widget(&username_input, layout[1]);
    f.render_widget(&path_input, layout[2]);
}

pub fn update(
    config: &mut AppConfig,
    model: &mut ConfigModel<'static>,
    state: &mut AppState,
    msg: ConfigMsg,
) {
    match msg {
        ConfigMsg::ToggleFocus => {
            model.active_field = match model.active_field {
                ConfigField::Username => ConfigField::DefaultPath,
                ConfigField::DefaultPath => ConfigField::Username,
            };
        }
        ConfigMsg::Save => {
            config.username = model.username_input.lines()[0].trim().to_string();
            let path_str = model.path_input.lines()[0].trim().to_string();
            config.default_dir = PathBuf::from(path_str);

            let _ = config.save();
            *state = AppState::Navigating;
        }
        ConfigMsg::Cancel => *state = AppState::Navigating,
        ConfigMsg::UpdateInput(ev) => {
            match model.active_field {
                ConfigField::Username => model.username_input.input(ev),
                ConfigField::DefaultPath => model.path_input.input(ev),
            };
        }
    }
}

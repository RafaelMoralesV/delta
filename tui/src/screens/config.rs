use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::TextArea;

use crate::{AppState, Message, config::AppConfig};

pub struct ConfigModel<'a> {
    pub text_area: TextArea<'a>,
}

pub enum ConfigMsg {
    UpdateInput(event::KeyEvent),
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
        _ => Some(ConfigMsg::UpdateInput(key).into()),
    }
}

impl<'a> ConfigModel<'a> {
    pub fn new(initial_name: String) -> Self {
        let mut text_area = TextArea::default();
        text_area.insert_str(initial_name);
        text_area.set_block(Block::default().borders(Borders::ALL).title(" User Name "));
        Self { text_area }
    }
}

pub fn view(model: &ConfigModel, f: &mut Frame, area: Rect) {
    // Center the config modal
    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Instructions
            Constraint::Length(3), // Input area
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new("Who is signing the bumps?").alignment(Alignment::Left),
        layout[0],
    );

    f.render_widget(&model.text_area, layout[1]);
}

pub fn update(
    config: &mut AppConfig,
    model: &mut ConfigModel<'static>,
    state: &mut AppState,
    msg: ConfigMsg,
) {
    match msg {
        ConfigMsg::Save => {
            config.username = model.text_area.lines()[0].trim().to_string();
            // We can also save the current nav directory as the new default
            // if you want that behavior, otherwise just keep the existing one.
            let _ = config.save();
            *state = AppState::Navigating;
        }
        ConfigMsg::Cancel => *state = AppState::Navigating,
        ConfigMsg::UpdateInput(ev) => {
            model.text_area.input(ev);
        }
    }
}

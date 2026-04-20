use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::TextArea;

use crate::{App, AppState};

pub struct ConfigModel<'a> {
    pub text_area: TextArea<'a>,
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

pub fn update(app: &mut App, event: event::KeyEvent, model: &mut ConfigModel<'static>) {
    let key = event.code;
    match key {
        KeyCode::Esc => app.state = AppState::Navigating,
        KeyCode::Enter => {
            app.config.username = model.text_area.lines()[0].trim().to_string();
            let _ = app.config.save();
            app.state = AppState::Navigating;
        }
        _ => {
            model.text_area.input(event);
        }
    }
}

use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

use crate::{App, AppState};

pub struct ProjectModel {
    pub name: String,
    pub versions: core::ProjectVersions,
}

pub fn view(model: &ProjectModel, f: &mut Frame, area: Rect) {
    let mut info = format!("Project: {}\n\n", model.name);

    let web_v = model
        .versions
        .web
        .map_or("Not Found".into(), |v| v.to_string());
    let web_n = model
        .versions
        .web
        .map_or("-".into(), |v| v.bumped().to_string());
    let bl_v = model
        .versions
        .bl
        .map_or("Not Found".into(), |v| v.to_string());
    let bl_n = model
        .versions
        .bl
        .map_or("-".into(), |v| v.bumped().to_string());

    info.push_str(&format!(
        "Web Version: {}\nWeb Next:    {}\n\n",
        web_v, web_n
    ));
    info.push_str(&format!("BL Version:  {}\nBL Next:     {}\n", bl_v, bl_n));

    let p = Paragraph::new(info).block(
        Block::default()
            .title(" Project Details ")
            .borders(Borders::ALL),
    );
    f.render_widget(p, area);
}

pub fn update(app: &mut App, event: event::KeyEvent) {
    let key = event.code;
    match key {
        KeyCode::Esc => app.state = AppState::Navigating,
        _ => {}
    }
}

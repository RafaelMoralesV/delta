use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};

use crate::{App, AppState, Message};

pub struct ProjectModel {
    pub name: String,
    pub versions: core::ProjectVersions,
}

pub enum ProjectMsg {
    Back,
    Bump,
}

impl From<ProjectMsg> for Message {
    fn from(v: ProjectMsg) -> Self {
        Message::Project(v)
    }
}

pub fn handle_key_event(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Esc => Some(ProjectMsg::Back.into()),
        KeyCode::Char('b') => Some(ProjectMsg::Bump.into()),
        _ => None,
    }
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

pub fn update(app: &mut App, msg: ProjectMsg) {
    match msg {
        ProjectMsg::Back => app.state = AppState::Navigating,
        ProjectMsg::Bump => {
            // Logic for bumping versions goes here.
            // Since we have mutable access to `app`, we can read `app.config.username`
            // and perform the IO required.
        }
    }
}

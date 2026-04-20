use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{App, AppState, Message};

pub enum Selection {
    Bl = 0,
    Web = 1,
}

pub struct ProjectModel {
    pub name: String,
    pub versions: core::ProjectVersions,
    pub selected: Selection,
    pub bump_message: String,
    pub is_naming_bump: bool, // Modal toggle
}

impl ProjectModel {
    pub fn new(name: String, versions: core::ProjectVersions) -> Self {
        Self {
            name,
            versions,
            selected: Selection::Web,
            bump_message: "".into(),
            is_naming_bump: false,
        }
    }
}

pub enum ProjectMsg {
    Back,
    MoveUp,
    MoveDown,
    PrepareBumpSingle,
    PrepareBumpBoth,
    UpdateInput(String),
    ConfirmBump,
    CancelBump,
}

impl From<ProjectMsg> for Message {
    fn from(v: ProjectMsg) -> Self {
        Message::Project(v)
    }
}

pub fn handle_key_event(key: event::KeyEvent, model: &ProjectModel) -> Option<Message> {
    if model.is_naming_bump {
        let msg = match key.code {
            KeyCode::Enter => ProjectMsg::ConfirmBump,
            KeyCode::Esc => ProjectMsg::CancelBump,
            KeyCode::Char(c) => ProjectMsg::UpdateInput(format!("{}{}", model.bump_message, c)),
            KeyCode::Backspace => {
                let mut s = model.bump_message.clone();
                s.pop();
                ProjectMsg::UpdateInput(s)
            }
            _ => return None,
        };

        return Some(msg.into());
    }

    let msg = match key.code {
        KeyCode::Esc => ProjectMsg::Back,
        KeyCode::Char('j') | KeyCode::Down => ProjectMsg::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => ProjectMsg::MoveUp,
        KeyCode::Char('b') => ProjectMsg::PrepareBumpSingle,
        KeyCode::Char('B') => ProjectMsg::PrepareBumpBoth,
        _ => return None,
    };

    Some(msg.into())
}

pub fn view(model: &ProjectModel, f: &mut Frame, area: Rect) {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(0),
        ])
        .split(area);

    // Styling for selection
    let web_style = if matches!(model.selected, Selection::Web) {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    let bl_style = if matches!(model.selected, Selection::Bl) {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default()
    };

    // Render Web Info
    let web_v = model
        .versions
        .web
        .as_ref()
        .map_or("-".to_string(), |v| v.to_string());
    f.render_widget(
        Paragraph::new(format!(" Web Version: {} -> {}", web_v, "Next"))
            .style(web_style)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT)),
        chunks[0],
    );

    // Render BL Info
    let bl_v = model
        .versions
        .bl
        .as_ref()
        .map_or("-".to_string(), |v| v.to_string());
    f.render_widget(
        Paragraph::new(format!(" BL Version:  {} -> {}", bl_v, "Next"))
            .style(bl_style)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)),
        chunks[1],
    );

    // --- MODAL LOGIC ---
    if model.is_naming_bump {
        let popup_area = centered_rect(60, 20, area);
        f.render_widget(Clear, popup_area); // This clears the background under the modal
        let block = Block::default()
            .title(" Bump Message ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let p = Paragraph::new(model.bump_message.as_str()).block(block);
        f.render_widget(p, popup_area);
    }
}

// Helper for centering the modal
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn update(app: &mut App, msg: ProjectMsg) {
    if let AppState::ViewingProject(ref mut model) = app.state {
        match msg {
            ProjectMsg::MoveUp => model.selected = Selection::Web,
            ProjectMsg::MoveDown => model.selected = Selection::Bl,
            ProjectMsg::PrepareBumpSingle | ProjectMsg::PrepareBumpBoth => {
                model.is_naming_bump = true;
                model.bump_message.clear();
            }
            ProjectMsg::UpdateInput(s) => model.bump_message = s,
            ProjectMsg::ConfirmBump => {
                // Perform actual IO here using model.bump_message
                model.is_naming_bump = false;
            }
            ProjectMsg::CancelBump => model.is_naming_bump = false,
            ProjectMsg::Back => app.state = AppState::Navigating,
        }
    }
}

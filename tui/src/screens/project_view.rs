use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{App, AppFocus, Message, config::AppConfig};

#[derive(Default)]
pub enum Selection {
    #[default]
    Bl = 0,
    Web = 1,
}

#[derive(Default)]
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
        KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Enter => ProjectMsg::PrepareBumpSingle,
        _ => return None,
    };

    Some(msg.into())
}

pub fn view(model: &ProjectModel, f: &mut Frame, area: Rect, cfg: &AppConfig) {
    let p = &cfg.palette;
    let title = format!("Proyecto: {}", model.name);

    crate::components::page::draw_page(
        f,
        area,
        &title,
        "B/Enter: Bump | Esc: Atrás",
        cfg,
        |f, chunk| {
            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(chunk);

            // Helper for the "next version" string with styling
            let render_version_line =
                |label: &str, current: &Option<core::AppVersion>, is_selected: bool| {
                    let current_str = current.as_ref().map_or("-".into(), |v| v.to_string());
                    let next_str = current
                        .as_ref()
                        .map(|v| v.bumped().to_string())
                        .unwrap_or_else(|| "-".into());

                    // Base16 logic: base05 (text) -> base0B (green/success for the new version)
                    let content = vec![
                        Span::raw(format!(" {} Version: ", label)),
                        Span::styled(current_str, Style::default().fg(p.get_color(&p.base05))),
                        Span::raw(" -> "),
                        Span::styled(
                            next_str,
                            Style::default()
                                .fg(p.get_color(&p.base0B))
                                .add_modifier(Modifier::BOLD),
                        ),
                    ];

                    let style = if is_selected {
                        Style::default().bg(p.get_color(&p.base02)) // Selection highlight
                    } else {
                        Style::default().fg(p.get_color(&p.base04)) // Dimmer text for inactive
                    };

                    Paragraph::new(Line::from(content)).style(style)
                };

            // Render Web Info
            f.render_widget(
                render_version_line(
                    "Web",
                    &model.versions.web,
                    matches!(model.selected, Selection::Web),
                )
                .block(
                    Block::default()
                        .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                        .border_style(Style::default().fg(p.get_color(&p.base03))),
                ),
                chunks[0],
            );

            // Render BL Info
            f.render_widget(
                render_version_line(
                    "BL",
                    &model.versions.bl,
                    matches!(model.selected, Selection::Bl),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(p.get_color(&p.base03))),
                ),
                chunks[1],
            );
        },
    );

    // --- Modal Logic ---
    if model.is_naming_bump {
        let popup_area = centered_rect(60, 20, area);
        f.render_widget(ratatui::widgets::Clear, popup_area);

        let block = Block::default()
            .title(" Mensaje ")
            .borders(Borders::ALL)
            // base09 (Orange) is great for "Action Required" modals
            .border_style(Style::default().fg(p.get_color(&p.base09)))
            .style(Style::default().bg(p.get_color(&p.base01))); // Slightly offset from main bg

        f.render_widget(
            Paragraph::new(model.bump_message.as_str())
                .block(block)
                .style(Style::default().fg(p.get_color(&p.base05))),
            popup_area,
        );
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
    let ref mut model = app.project_view_model;
    if let AppFocus::ViewingProject = app.current_focused_screen {
        match msg {
            ProjectMsg::MoveUp => model.selected = Selection::Web,
            ProjectMsg::MoveDown => model.selected = Selection::Bl,
            ProjectMsg::PrepareBumpSingle => {
                model.is_naming_bump = true;
                model.bump_message.clear();
            }
            ProjectMsg::UpdateInput(s) => model.bump_message = s,
            ProjectMsg::ConfirmBump => {
                let instructions = core::BumpInstructions {
                    author: app.config.username.clone(),
                    project_dir: app.navigation_state.current_dir.clone(),
                    project_name: model.name.clone(),
                    description: model.bump_message.clone(),
                };

                match model.selected {
                    Selection::Web => {
                        if let Some(current) = model.versions.web {
                            let _ = core::execute_bump(&app.fs, instructions, current, true);
                        }
                    }
                    Selection::Bl => {
                        if let Some(current) = model.versions.bl {
                            let _ = core::execute_bump(&app.fs, instructions, current, false);
                        }
                    }
                }

                // Refresh versions after bump so the UI stays in sync
                model.versions = core::get_current_versions(
                    &app.fs,
                    &app.config.default_dir.join(&model.name),
                    &model.name,
                );
                model.is_naming_bump = false;
            }
            ProjectMsg::CancelBump => model.is_naming_bump = false,
            ProjectMsg::Back => app.current_focused_screen = AppFocus::Navigating,
        }
    }
}

use delta_core::{FileHandler, navigation::DirectoryDisplayInfo};
use ratatui_textarea::TextArea;
use ratatui_textarea::WrapMode;
use std::path::PathBuf;

use ratatui::{
    Frame,
    crossterm::event::{self, KeyCode},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    App, AppFocus, Message,
    components::layout::{AppLayout, Modal, Panel},
    config::AppConfig,
};

#[derive(Default)]
pub enum Selection {
    #[default]
    Bl = 0,
    Web = 1,
}

#[derive(Default)]
pub struct ProjectModel {
    pub name: String,
    pub versions: delta_core::ProjectVersions,
    pub selected: Selection,
    pub bump_message: TextArea<'static>,
    pub name_input: TextArea<'static>,
    pub bump_focus: BumpFocus,
    pub is_naming_bump: bool,
    pub is_git_repository: bool,
    pub is_unknown_directory: bool,
}

#[derive(Default, PartialEq)]
pub enum BumpFocus {
    #[default]
    Description,
    Name,
}

impl ProjectModel {
    pub fn new(name: String, versions: delta_core::ProjectVersions) -> Self {
        Self {
            name,
            versions,
            is_naming_bump: false,
            is_git_repository: false,
            is_unknown_directory: false,
            ..Default::default()
        }
    }

    pub fn from_display_info(
        name: String,
        info: DirectoryDisplayInfo,
        path: PathBuf,
        fs: &impl FileHandler,
    ) -> Self {
        use delta_core::navigation::ProjectType;

        match info.project_type {
            ProjectType::FullStack | ProjectType::Web | ProjectType::BusinessLayer => {
                let versions = delta_core::get_current_versions(fs, &path, &name);
                Self::new(name, versions)
            }
            ProjectType::GitRepository => {
                let mut model = Self::new(name, Default::default());
                model.is_git_repository = true;
                model
            }
            ProjectType::Unknown => {
                let mut model = Self::new(name, Default::default());
                model.is_unknown_directory = true;
                model
            }
        }
    }
}

pub enum ProjectMsg {
    Back,
    MoveUp,
    MoveDown,
    PrepareBumpSingle,
    ConfirmBump,
    CancelBump,
    ToggleBumpFocus,
    TextAreaInput(event::KeyEvent),
    NameInput(event::KeyEvent),
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
            KeyCode::Tab => ProjectMsg::ToggleBumpFocus,
            _ => match model.bump_focus {
                BumpFocus::Description => ProjectMsg::TextAreaInput(key),
                BumpFocus::Name => ProjectMsg::NameInput(key),
            },
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

pub fn view(state: &App, f: &mut Frame, area: Rect) {
    let model = &state.project_view_model;
    let cfg = &state.config;

    let title = format!("Proyecto: {}", model.name);
    let p = &cfg.palette;

    let layout = AppLayout::new(
        title,
        "",
        matches!(state.current_focused_screen, AppFocus::ViewingProject),
    );

    layout.render(f, area, cfg, |f, inner| {
        let panel = Panel::new();

        panel.render(f, inner, cfg, |f, chunk| {
            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(chunk);

            // Helper for the "next version" string with styling
            let render_version_line =
                |label: &str, current: &Option<delta_core::AppVersion>, is_selected: bool| {
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
        });

        // --- Modal Logic ---
        if model.is_naming_bump {
            let modal = Modal::new("Bump");

            const NAME_BOX_HEIGHT: u16 = 3; // 1 line + top/bottom border
            const MIN_DESC_ROWS: u16 = 5;
            const MAX_DESC_ROWS: u16 = 10;
            const MODAL_BORDER: u16 = 2; // Modal's own outer top/bottom border

            let content_width = modal.inner_width(area).saturating_sub(2);
            let desc_rows = wrapped_row_count(&model.bump_message, content_width)
                .clamp(MIN_DESC_ROWS, MAX_DESC_ROWS);
            let desc_box_height = desc_rows + 2; // + top/bottom border

            let total_height = NAME_BOX_HEIGHT + desc_box_height + MODAL_BORDER;

            modal.render_dynamic(f, area, cfg, total_height, |f, popup_area| {
                let rows = Layout::vertical([
                    Constraint::Length(NAME_BOX_HEIGHT),
                    Constraint::Length(desc_box_height),
                ])
                .split(popup_area);

                let name_style = if model.bump_focus == BumpFocus::Name {
                    Style::default().fg(p.get_color(&p.base0B))
                } else {
                    Style::default().fg(p.get_color(&p.base03))
                };
                let desc_style = if model.bump_focus == BumpFocus::Description {
                    Style::default().fg(p.get_color(&p.base0B))
                } else {
                    Style::default().fg(p.get_color(&p.base03))
                };

                let name_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Nombre (Tab para editar)")
                    .border_style(name_style);
                let desc_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Mensaje")
                    .border_style(desc_style);

                f.render_widget(&model.name_input, name_block.inner(rows[0]));
                f.render_widget(name_block, rows[0]);

                f.render_widget(&model.bump_message, desc_block.inner(rows[1]));
                f.render_widget(desc_block, rows[1]);
            });
        }
    });
}

pub fn view_placeholder(f: &mut Frame, area: Rect, cfg: &AppConfig) {
    let p = &cfg.palette;
    let layout = AppLayout::new(
        "Detalles del proyecto",
        "Selecciona un proyecto para ver sus detalles",
        false,
    );

    layout.render(f, area, cfg, |f, chunk| {
        let panel = Panel::new();

        panel.render(f, chunk, cfg, |f, inner| {
            let text = vec![
                Line::from(Span::styled(
                    "Sin seleccion",
                    Style::default()
                        .fg(p.get_color(&p.base04))
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Usa las flechas para navegar y enter para seleccionar,",
                    Style::default().fg(p.get_color(&p.base03)),
                )),
                Line::from(Span::styled(
                    "ver detalles y subir su version",
                    Style::default().fg(p.get_color(&p.base03)),
                )),
            ];

            let paragraph = Paragraph::new(text).alignment(Alignment::Center);

            f.render_widget(paragraph, inner);
        });
    });
}

pub fn update(app: &mut App, msg: ProjectMsg) {
    let ref mut model = app.project_view_model;
    if let AppFocus::ViewingProject = app.current_focused_screen {
        match msg {
            ProjectMsg::MoveUp => model.selected = Selection::Web,
            ProjectMsg::MoveDown => model.selected = Selection::Bl,
            ProjectMsg::PrepareBumpSingle => {
                model.is_naming_bump = true;
                model.bump_message = TextArea::default();
                model.bump_message.set_wrap_mode(WrapMode::WordOrGlyph);
                model.name_input = TextArea::new(vec![model.name.clone()]);
                model
                    .name_input
                    .move_cursor(ratatui_textarea::CursorMove::End);
                model.bump_focus = BumpFocus::Description;
            }
            ProjectMsg::ToggleBumpFocus => {
                model.bump_focus = match model.bump_focus {
                    BumpFocus::Description => BumpFocus::Name,
                    BumpFocus::Name => BumpFocus::Description,
                };
            }
            ProjectMsg::TextAreaInput(key) => {
                model.bump_message.input(key);
            }
            ProjectMsg::NameInput(key) => {
                model.name_input.input(key);
            }
            ProjectMsg::ConfirmBump => {
                let object = model.name_input.lines().join("");

                match model.selected {
                    Selection::Web => {
                        if let Some(current) = model.versions.web {
                            let instructions = delta_core::BumpInstructions {
                                author: app.config.username.clone(),
                                project_dir: app.navigation_state.current_dir.clone(),
                                object: object.clone(),
                                project_name: model.name.clone(),
                                description: model.bump_message.lines().join("\n"),
                                version_original: current,
                                is_web_project: true,
                            };
                            let _ = instructions.execute_bump(&app.fs);
                        }
                    }
                    Selection::Bl => {
                        if let Some(current) = model.versions.bl {
                            let instructions = delta_core::BumpInstructions {
                                author: app.config.username.clone(),
                                project_dir: app.navigation_state.current_dir.clone(),
                                object: object.clone(),
                                project_name: model.name.clone(),
                                description: model.bump_message.lines().join("\n"),
                                version_original: current,
                                is_web_project: false,
                            };
                            let _ = instructions.execute_bump(&app.fs);
                        }
                    }
                }

                model.versions = delta_core::get_current_versions(
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

/// Approximate number of *visual* rows `WrapMode::WordOrGlyph` will render
/// this textarea's content into, given an inner width in columns.
fn wrapped_row_count(textarea: &TextArea, width: u16) -> u16 {
    let width = width.max(1) as usize;
    let rows: usize = textarea
        .lines()
        .iter()
        .map(|line| {
            let len = line.chars().count();
            if len == 0 {
                1
            } else {
                (len + width - 1) / width // ceil division
            }
        })
        .sum();
    rows.max(1) as u16
}

mod config;
mod screens;

use crate::config::AppConfig;
use crate::screens::config::ConfigModel;
use crate::screens::navigation::NavModel;
use crate::screens::project_view::ProjectModel;

use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    widgets::Paragraph,
};
use std::{env, error::Error, io, path::PathBuf};

pub enum AppState {
    Navigating,
    ViewingProject(ProjectModel),
    Configuring(ConfigModel<'static>),
}

struct App {
    state: AppState,
    navigation_state: NavModel,
    config: AppConfig,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config = AppConfig::load();
        Self {
            state: AppState::Navigating,
            navigation_state: NavModel::new(root),
            config,
            should_quit: false,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    while !app.should_quit {
        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(2)
                    .constraints([Constraint::Min(1), Constraint::Length(3)])
                    .split(f.area());

                match &mut app.state {
                    AppState::Navigating => {
                        screens::navigation::view(&mut app.navigation_state, f, chunks[0]);
                        f.render_widget(
                            Paragraph::new("↑/↓: Navegar | Enter: Abrir | C: Config | Esc: Salir"),
                            chunks[1],
                        );
                    }
                    AppState::ViewingProject(m) => {
                        screens::project_view::view(m, f, chunks[0]);
                        f.render_widget(Paragraph::new("B: Bump | Esc: Atrás"), chunks[1]);
                    }
                    AppState::Configuring(m) => {
                        screens::config::view(m, f, chunks[0]);
                        f.render_widget(
                            Paragraph::new("Enter: Guardar | Esc: Cancelar"),
                            chunks[1],
                        );
                    }
                }
            })
            .unwrap();

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            update(app, key);
        }
    }
    Ok(())
}

fn update(app: &mut App, event: event::KeyEvent) {
    let key = event.code;
    match &mut app.state {
        AppState::Navigating => match key {
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('c') => {
                app.state = AppState::Configuring(ConfigModel::new(app.config.username.clone()));
            }
            KeyCode::Up => app.navigation_state.previous(),
            KeyCode::Down => app.navigation_state.next(),
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(i) = app.navigation_state.list_state.selected() {
                    let selected = app.navigation_state.items[i].clone();
                    if selected == ".. (Up)" {
                        app.navigation_state.current_dir.pop();
                        app.navigation_state.load_directory();
                    } else {
                        let new_path = app.navigation_state.current_dir.join(&selected);
                        if key == KeyCode::Char(' ') {
                            if new_path.is_dir() {
                                app.navigation_state.current_dir = new_path;
                                app.navigation_state.load_directory();
                            }
                        } else {
                            let versions = core::get_current_versions(&new_path, &selected);
                            app.state = AppState::ViewingProject(ProjectModel {
                                name: selected,
                                versions,
                            });
                        }
                    }
                }
            }
            _ => {}
        },
        AppState::ViewingProject(_) => match key {
            KeyCode::Esc => app.state = AppState::Navigating,
            _ => {}
        },
        AppState::Configuring(m) => match key {
            KeyCode::Esc => app.state = AppState::Navigating,
            KeyCode::Enter => {
                app.config.username = m.text_area.lines()[0].trim().to_string();
                let _ = app.config.save();
                app.state = AppState::Navigating;
            }
            _ => {
                m.text_area.input(event);
            }
        },
    }
}

mod screens;

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
}

struct App {
    state: AppState,
    navigation_state: NavModel,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            state: AppState::Navigating,
            navigation_state: NavModel::new(root),
            should_quit: false,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup Terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // Restore Terminal
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
                        let m = &mut app.navigation_state;
                        screens::navigation::view(m, f, chunks[0]);
                        f.render_widget(
                            Paragraph::new("↑/↓: Moverse | Espacio: Entrar en carpeta | Enter: Seleccionar | Esc: Salir"),
                            chunks[1],
                        );
                    }
                    AppState::ViewingProject(m) => {
                        screens::project_view::view(m, f, chunks[0]);
                        f.render_widget(Paragraph::new("B: Bump | Esc: Atrás"), chunks[1]);
                    }
                }
            })
            .unwrap();

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            update(app, key.code);
        }
    }
    Ok(())
}

fn update(app: &mut App, key: KeyCode) {
    let m = &mut app.navigation_state;
    match &mut app.state {
        AppState::Navigating => match key {
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Up => m.previous(),
            KeyCode::Down => m.next(),
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(i) = m.list_state.selected() {
                    let selected = m.items[i].clone();
                    if selected == ".. (Up)" {
                        m.current_dir.pop();
                        m.load_directory();
                    } else {
                        let new_path = m.current_dir.join(&selected);
                        if key == KeyCode::Char(' ') {
                            if new_path.is_dir() {
                                m.current_dir = new_path;
                                m.load_directory();
                            }
                        } else {
                            // Enter: Load project
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
        AppState::ViewingProject(_m) => match key {
            KeyCode::Esc => {
                app.state = AppState::Navigating;
            }
            KeyCode::Char('b') => { /* Handle Bump */ }
            _ => {}
        },
    }
}

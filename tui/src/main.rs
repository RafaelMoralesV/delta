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
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
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
    match &mut app.state {
        AppState::Navigating => screens::navigation::update(app, event),
        AppState::ViewingProject(_) => screens::project_view::update(app, event),
        AppState::Configuring(m) => screens::config::update(app, event, m),
    }
}

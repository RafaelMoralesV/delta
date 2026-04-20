mod config;
mod screens;

use crate::screens::config::ConfigModel;
use crate::screens::navigation::NavModel;
use crate::screens::project_view::ProjectModel;
use crate::{config::AppConfig, screens::project_view::ProjectMsg};

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
use std::{error::Error, io};

pub enum AppState {
    Navigating,
    ViewingProject(ProjectModel),
    Configuring,
}

struct App {
    state: AppState,
    navigation_state: NavModel,
    config: AppConfig,
    config_model: ConfigModel<'static>,
    should_quit: bool,
}

pub enum Message {
    // Global Actions
    Quit,

    // Scoped Actions (using From/Into)
    Navigation(screens::navigation::NavMsg),
    Config(screens::config::ConfigMsg),
    Project(ProjectMsg),
}

impl App {
    fn new() -> Self {
        let config = AppConfig::load();
        let nav_state = NavModel::new(config.default_dir.clone());
        let config_model = ConfigModel::new(config.username.clone(), config.default_dir.clone());

        Self {
            state: AppState::Navigating,
            navigation_state: nav_state,
            config_model,
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
                    AppState::Configuring => {
                        screens::config::view(&app.config_model, f, chunks[0]);
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

            let msg = match &app.state {
                AppState::Navigating => screens::navigation::handle_key_event(key),
                AppState::Configuring => screens::config::handle_key_event(key),
                AppState::ViewingProject(_) => screens::project_view::handle_key_event(key),
            };

            if let Some(m) = msg {
                update(app, m);
            }
        }
    }
    Ok(())
}

fn update(app: &mut App, msg: Message) {
    match msg {
        Message::Quit => app.should_quit = true,
        Message::Navigation(sub_msg) => screens::navigation::update(app, sub_msg),
        Message::Config(sub_msg) => {
            screens::config::update(app, sub_msg);
        }
        Message::Project(sub_msg) => screens::project_view::update(app, sub_msg),
    }
}

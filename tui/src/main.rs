mod components;
mod config;
mod screens;

use crate::screens::config::ConfigModel;
use crate::screens::navigation::NavModel;
use crate::screens::project_view::ProjectModel;
use crate::{config::AppConfig, screens::project_view::ProjectMsg};

use core::OsFileSystem;
use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::event::{KeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use std::{error::Error, io};

pub enum AppFocus {
    Navigating,
    ViewingProject,
    Configuring,
}

struct App {
    /// Pantalla en la cual estamos actualmente
    current_focused_screen: AppFocus,

    /// Configuracion de la App.
    config: AppConfig,

    /// Estado de la pantalla de navegacion
    navigation_state: NavModel,

    /// Estado de la pantalla de vision de proyecto
    project_view_model: ProjectModel,

    /// Estado de la pantalla de configuracion
    config_model: ConfigModel<'static>,

    /// Cierra la sesion
    should_quit: bool,

    /// Helper sencillo para no andar creandolo a cada rato. No mantiene estado.
    fs: OsFileSystem,
}

pub enum Message {
    // Global Actions
    Quit,

    // Mensajes de la pantalla de navegacion
    Navigation(screens::navigation::NavMsg),

    // Mensajes de la pantalla de configuracion
    Config(screens::config::ConfigMsg),

    // Mensajes de la pantalla de vision de proyecto
    Project(ProjectMsg),
}

impl App {
    fn new() -> Self {
        let fs = OsFileSystem;
        let config = AppConfig::load();
        let nav_state = NavModel::new(&fs, config.default_dir.clone());
        let config_model = ConfigModel::new(config.username.clone(), config.default_dir.clone());

        Self {
            current_focused_screen: AppFocus::Navigating,
            navigation_state: nav_state,
            config_model,
            project_view_model: ProjectModel::default(),
            config,
            should_quit: false,
            fs,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Necesario para soportar el backslash del teclado latino
    let _ = std::io::stdout().execute(PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
    ));

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
                let main_area = f.area();

                if main_area.width > 80 {
                    // Seccionamos por mitades la pantalla.
                }

                match &mut app.current_focused_screen {
                    AppFocus::Navigating => {
                        screens::navigation::view(
                            &mut app.navigation_state,
                            f,
                            main_area,
                            &app.config,
                        );
                    }
                    AppFocus::ViewingProject => {
                        screens::project_view::view(
                            &app.project_view_model,
                            f,
                            main_area,
                            &app.config,
                        );
                    }
                    AppFocus::Configuring => {
                        screens::config::view(&app.config_model, f, main_area, &app.config);
                    }
                }
            })
            .unwrap();

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }

            let msg = match &app.current_focused_screen {
                AppFocus::Navigating => {
                    screens::navigation::handle_key_event(key, &app.navigation_state)
                }
                AppFocus::Configuring => screens::config::handle_key_event(key),
                AppFocus::ViewingProject => {
                    screens::project_view::handle_key_event(key, &app.project_view_model)
                }
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

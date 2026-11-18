mod cli;
mod components;
mod config;
mod screens;

use crate::screens::config::ConfigModel;
use crate::screens::navigation::NavModel;
use crate::screens::project_view::ProjectModel;
use crate::{config::AppConfig, screens::project_view::ProjectMsg};

use delta_core::OsFileSystem;
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
    current_focused_screen: AppFocus,
    config: AppConfig,
    navigation_state: NavModel,
    project_view_model: ProjectModel,
    config_model: ConfigModel<'static>,
    should_quit: bool,
    fs: OsFileSystem,
}

pub enum Message {
    Quit,
    Navigation(screens::navigation::NavMsg),
    Config(screens::config::ConfigMsg),
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
    // Con argumentos: modo CLI (no interactivo). Sin argumentos: TUI.
    if std::env::args_os().len() > 1 {
        std::process::exit(cli::run());
    }

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
                    // Split screen layout
                    let chunks = ratatui::layout::Layout::horizontal([
                        ratatui::layout::Constraint::Percentage(40),
                        ratatui::layout::Constraint::Percentage(60),
                    ])
                    .split(main_area);

                    // Always show navigation on left
                    screens::navigation::view(app, f, chunks[0]);

                    // Show appropriate view on right
                    match &app.current_focused_screen {
                        AppFocus::ViewingProject => {
                            screens::project_view::view(&app, f, chunks[1]);
                        }
                        AppFocus::Configuring => {
                            screens::config::view(&app.config_model, f, chunks[1], &app.config);
                        }
                        AppFocus::Navigating => {
                            // Show preview of selected item
                            if let Some((name, info, path)) =
                                app.navigation_state.get_selected_preview()
                            {
                                app.project_view_model =
                                    ProjectModel::from_display_info(name, info, path, &app.fs);
                                screens::project_view::view(&app, f, chunks[1]);
                            } else {
                                screens::project_view::view_placeholder(f, chunks[1], &app.config);
                            }
                        }
                    }
                } else {
                    // Narrow screen - full screen mode
                    match &mut app.current_focused_screen {
                        AppFocus::Navigating => {
                            screens::navigation::view(app, f, main_area);
                        }
                        AppFocus::ViewingProject => {
                            screens::project_view::view(&app, f, main_area);
                        }
                        AppFocus::Configuring => {
                            screens::config::view(&app.config_model, f, main_area, &app.config);
                        }
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

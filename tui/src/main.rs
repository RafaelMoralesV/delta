use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{env, error::Error, fs, io, path::PathBuf};

// Assuming you exported these from your core crate
// use core::{get_current_versions, ProjectVersions, AppVersion};

enum AppState {
    Navigating,
    ViewingProject(String), // Holds the project name
}

struct App {
    state: AppState,
    current_dir: PathBuf,
    items: Vec<String>,
    list_state: ListState,
    // Store versions temporarily when viewing a project
    project_versions: Option<core::ProjectVersions>,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            state: AppState::Navigating,
            current_dir: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            items: vec![],
            list_state: ListState::default(),
            project_versions: None,
        };
        app.load_directory();
        app
    }

    fn load_directory(&mut self) {
        self.items.clear();
        self.items.push(".. (Up)".to_string());

        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            let mut dirs: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            dirs.sort();
            self.items.extend(dirs);
        }
        self.list_state.select(Some(0));
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
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
    loop {
        terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(2)
                    .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                    .split(f.area());

                match &app.state {
                    AppState::Navigating => {
                        let items: Vec<ListItem> = app
                            .items
                            .iter()
                            .map(|i| ListItem::new(i.as_str()))
                            .collect();

                        let list = List::new(items)
                            .block(
                                Block::default()
                                    .title(format!(" Navigating: {} ", app.current_dir.display()))
                                    .borders(Borders::ALL),
                            )
                            .highlight_style(
                                Style::default()
                                    .bg(Color::DarkGray)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .highlight_symbol(">> ");

                        f.render_stateful_widget(list, chunks[0], &mut app.list_state);

                        let instructions = Paragraph::new("↑/↓: Move | Enter: Select | Esc: Quit");
                        f.render_widget(instructions, chunks[1]);
                    }
                    AppState::ViewingProject(project_name) => {
                        let mut info = format!("Project: {}\n\n", project_name);

                        if let Some(versions) = &app.project_versions {
                            info.push_str(&format!(
                                "Web Version: {}\n",
                                versions
                                    .web
                                    .map_or("Not Found".to_string(), |v| v.to_string())
                            ));
                            info.push_str(&format!(
                                "Web Next:    {}\n\n",
                                versions
                                    .web
                                    .map_or("-".to_string(), |v| v.bumped().to_string())
                            ));

                            info.push_str(&format!(
                                "BL Version:  {}\n",
                                versions
                                    .bl
                                    .map_or("Not Found".to_string(), |v| v.to_string())
                            ));
                            info.push_str(&format!(
                                "BL Next:     {}\n",
                                versions
                                    .bl
                                    .map_or("-".to_string(), |v| v.bumped().to_string())
                            ));
                        }

                        let p = Paragraph::new(info).block(
                            Block::default()
                                .title(" Project Details ")
                                .borders(Borders::ALL),
                        );
                        f.render_widget(p, chunks[0]);

                        let instructions =
                            Paragraph::new("B: Bump Versions | Esc: Back to Navigation");
                        f.render_widget(instructions, chunks[1]);
                    }
                }
            })
            .unwrap();

        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }

            match app.state {
                AppState::Navigating => match key.code {
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Up => app.previous(),
                    KeyCode::Down => app.next(),
                    KeyCode::Char(' ') => {
                        if let Some(i) = app.list_state.selected() {
                            let selected = &app.items[i];
                            if selected == ".. (Up)" {
                                app.current_dir.pop();
                            } else {
                                let new_path = app.current_dir.join(selected);
                                if new_path.is_dir() {
                                    app.current_dir = new_path;
                                }
                            }
                            app.load_directory();
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(i) = app.list_state.selected() {
                            let selected = &app.items[i];
                            if selected == ".. (Up)" {
                                app.current_dir.pop();
                                app.load_directory();
                            } else {
                                // Assume they selected a project folder like "FacOpeAprobacion"
                                let new_path = app.current_dir.join(selected);
                                app.current_dir = new_path;

                                let project_versions =
                                    core::get_current_versions(&app.current_dir, selected);

                                app.project_versions = Some(project_versions);
                                app.state = AppState::ViewingProject(selected.clone());
                            }
                        }
                    }
                    _ => {}
                },
                AppState::ViewingProject(_) => match key.code {
                    KeyCode::Esc => {
                        // Go back out
                        app.current_dir.pop();
                        app.load_directory();
                        app.project_versions = None;
                        app.state = AppState::Navigating;
                    }
                    KeyCode::Char('b') => {
                        // TODO: Transition to `AppState::InputtingMetadata` to use
                        // `ratatui-textarea` to gather Name, Object, Description
                        // and finally write the bumped versions to disk.
                    }
                    _ => {}
                },
            }
        }
    }
}

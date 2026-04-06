use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::TextArea;
use std::{error::Error, io};

// Import your core library
// use core::{BumpMetadata, ProjectType, bump_project};

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let res = run_app(&mut terminal);

    // Restore terminal
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    // Set up a basic input area for the Version number
    let mut version_textarea = TextArea::default();
    version_textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Enter Version (e.g., 8.0.5.0) "),
    );

    loop {
        terminal
            .draw(|f| {
                // Split the screen
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(2)
                    .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                    .split(f.area());

                // Render the input box
                f.render_widget(&version_textarea, chunks[0]);

                // Render instructions below
                let instructions = Paragraph::new("Press Esc to quit. Press Enter to submit.");
                f.render_widget(instructions, chunks[1]);
            })
            .unwrap();

        // Handle Input
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Esc => break, // Exit
                KeyCode::Enter => {
                    // Extract data from inputs, build `BumpMetadata`,
                    // and call `core::bump_project()`
                    let _version_input = version_textarea.lines()[0].clone();

                    // break or show success message
                }
                _ => {
                    // Feed keypresses into the textarea
                    version_textarea.input(key);
                }
            }
        }
    }
    Ok(())
}

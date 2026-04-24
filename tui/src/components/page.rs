use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

/// A reusable wrapper to provide a consistent look for all screens
pub fn draw_page<F>(f: &mut Frame, area: Rect, title: &str, footer: &str, content_fn: F)
where
    F: FnOnce(&mut Frame, Rect),
{
    let chunks = Layout::vertical([
        Constraint::Min(0),    // Content
        Constraint::Length(1), // Standard Footer
    ])
    .split(area);

    // Global Style: Let's say every page has a specific background or a main border
    let main_block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    // Render the content within the block's inner area
    let inner_area = main_block.inner(chunks[0]);
    f.render_widget(main_block, chunks[0]);

    // Call the specific screen logic
    content_fn(f, inner_area);

    // Global Footer Style
    let footer_widget = Paragraph::new(footer)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Indexed(236)).fg(Color::Gray));
    f.render_widget(footer_widget, chunks[1]);
}

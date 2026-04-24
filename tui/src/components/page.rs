use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

use crate::config::AppConfig;

/// Updated to accept the AppConfig for dynamic Base16 styling
pub fn draw_page<F>(
    f: &mut Frame,
    area: Rect,
    title: &str,
    footer: &str,
    config: &AppConfig,
    content_fn: F,
) where
    F: FnOnce(&mut Frame, Rect),
{
    let p = &config.palette;

    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    // Use base0D (usually Blue/Accent) for borders
    // and base00 for the main background
    let main_block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(p.get_color(&p.base0D)))
        .style(Style::default().bg(p.get_color(&p.base00)));

    let inner_area = main_block.inner(chunks[0]);
    f.render_widget(main_block, chunks[0]);

    // Call the specific screen logic
    content_fn(f, inner_area);

    // Global Footer Style:
    // Using base01 (Lighter Background) and base04 (Dark Foreground/Grey)
    let footer_widget = Paragraph::new(footer).alignment(Alignment::Center).style(
        Style::default()
            .bg(p.get_color(&p.base01))
            .fg(p.get_color(&p.base04)),
    );

    f.render_widget(footer_widget, chunks[1]);
}

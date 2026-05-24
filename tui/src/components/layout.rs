use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::config::AppConfig;

pub struct AppLayout {
    pub title: String,
    pub footer: String,
}

impl AppLayout {
    pub fn new(title: impl Into<String>, footer: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            footer: footer.into(),
        }
    }

    /// Render a full-screen layout with the given content function
    pub fn render<F>(&self, f: &mut Frame, area: Rect, config: &AppConfig, content: F)
    where
        F: FnOnce(&mut Frame, Rect),
    {
        let p = &config.palette;

        // Create the main layout
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

        // Render the main block with border
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.get_color(&p.base0D)))
            .style(Style::default().bg(p.get_color(&p.base00)));

        let inner_area = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);

        // Render the content
        content(f, inner_area);

        // Render the footer
        let footer = Paragraph::new(self.footer.as_str())
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .bg(p.get_color(&p.base01))
                    .fg(p.get_color(&p.base04)),
            );

        f.render_widget(footer, chunks[1]);
    }
}

/// Simple panel with border and optional title
pub struct Panel {
    pub title: Option<String>,
    pub borders: Borders,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            title: None,
            borders: Borders::ALL,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn render<F>(&self, f: &mut Frame, area: Rect, config: &AppConfig, content: F)
    where
        F: FnOnce(&mut Frame, Rect),
    {
        let p = &config.palette;

        let mut block = Block::default()
            .borders(self.borders)
            .border_style(Style::default().fg(p.get_color(&p.base03)));

        if let Some(title) = &self.title {
            block = block.title(format!(" {} ", title));
        }

        let inner_area = block.inner(area);
        f.render_widget(block, area);
        content(f, inner_area);
    }
}

/// Modal dialog helper
pub struct Modal {
    pub title: String,
    pub width_percent: u16,
    pub height_percent: u16,
}

impl Modal {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width_percent: 60,
            height_percent: 20,
        }
    }

    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent;
        self.height_percent = height_percent;
        self
    }

    pub fn render<F>(&self, f: &mut Frame, area: Rect, config: &AppConfig, content: F)
    where
        F: FnOnce(&mut Frame, Rect),
    {
        let p = &config.palette;

        // Calculate centered area
        let popup_area = self.centered_rect(area);

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Create the modal block
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(p.get_color(&p.base09)))
            .style(Style::default().bg(p.get_color(&p.base01)));

        let inner_area = block.inner(popup_area);
        f.render_widget(block, popup_area);
        content(f, inner_area);
    }

    fn centered_rect(&self, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - self.height_percent) / 2),
                Constraint::Percentage(self.height_percent),
                Constraint::Percentage((100 - self.height_percent) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - self.width_percent) / 2),
                Constraint::Percentage(self.width_percent),
                Constraint::Percentage((100 - self.width_percent) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

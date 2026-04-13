use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::{fs, path::PathBuf};

pub struct NavModel {
    pub current_dir: PathBuf,
    pub items: Vec<String>,
    pub list_state: ListState,
}

impl NavModel {
    pub fn new(path: PathBuf) -> Self {
        let mut model = Self {
            current_dir: path,
            items: vec![],
            list_state: ListState::default(),
        };
        model.load_directory();
        model
    }

    pub fn load_directory(&mut self) {
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

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

pub fn view(model: &mut NavModel, f: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = model
        .items
        .iter()
        .map(|i| ListItem::new(i.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" Navigating: {} ", model.current_dir.display()))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut model.list_state);
}

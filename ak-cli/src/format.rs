use chrono::{DateTime, Utc};
use pbjson_types::Timestamp;
use ratatui::style::{Color, Modifier, Style};

pub use ak_platform::tui::render_json;

// Styles — use functions instead of lazy statics for composability
pub fn key_style() -> Style {
    Style::default().fg(Color::Cyan) // ANSI color 6
}

pub fn value_style() -> Style {
    Style::default().fg(Color::Green) // ANSI color 2
}

pub fn box_style() -> Style {
    Style::default()
        .fg(Color::Rgb(250, 250, 250))
        .bg(Color::Rgb(253, 75, 45))
        .add_modifier(Modifier::BOLD)
}

pub fn inline_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

pub fn render_timestamp(ot: Option<Timestamp>) -> String {
    match ot {
        Some(t) => {
            let dt: DateTime<Utc> = match t.try_into() {
                Ok(date) => date,
                Err(e) => return e.to_string(),
            };
            dt.to_rfc2822()
        }
        None => "-".to_string(),
    }
}

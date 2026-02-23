use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::tui::commands::Command;

pub fn render(commands: &[Command], selected_index: usize, total_shown: usize) -> Paragraph<'static> {
    let visible_count = total_shown.min(commands.len());
    let start = if selected_index >= visible_count {
        selected_index - visible_count + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = commands
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_count)
        .map(|(i, cmd)| {
            let is_selected = i == selected_index;

            let name = if is_selected {
                Span::styled(
                    format!("  {:<20}", cmd.name),
                    Style::default()
                        .fg(Color::Rgb(180, 130, 255))
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!("  {:<20}", cmd.name),
                    Style::default().fg(Color::Rgb(160, 160, 175)),
                )
            };

            let desc = Span::styled(
                cmd.description.to_string(),
                Style::default().fg(Color::Rgb(90, 90, 105)),
            );

            Line::from(vec![name, desc])
        })
        .collect();

    // Pagination hint
    if commands.len() > visible_count {
        lines.push(Line::from(Span::styled(
            format!(
                "  ▼  ({}/{})",
                (start + visible_count).min(commands.len()),
                commands.len()
            ),
            Style::default().fg(Color::Rgb(80, 80, 90)),
        )));
    }

    Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 65)))
            .style(Style::default().bg(Color::Rgb(14, 14, 20))),
    )
}
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::tui::app::{OutputBlock, OutputKind};

pub fn render_lines(blocks: &[OutputBlock]) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for block in blocks {
        match block.kind {
            OutputKind::Command => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "> ".to_string(),
                        Style::default()
                            .fg(Color::Rgb(160, 120, 220))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        block.content.clone(),
                        Style::default()
                            .fg(Color::Rgb(200, 200, 210))
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                // Spacer
                lines.push(Line::from(""));
            }

            OutputKind::Success => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "✔ ".to_string(),
                        Style::default()
                            .fg(Color::Rgb(100, 220, 120))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        block.content.clone(),
                        Style::default().fg(Color::Rgb(160, 230, 170)),
                    ),
                ]));
                lines.push(Line::from(""));
            }

            OutputKind::Error => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "✖ ".to_string(),
                        Style::default()
                            .fg(Color::Rgb(220, 80, 80))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        block.content.clone(),
                        Style::default().fg(Color::Rgb(230, 100, 100)),
                    ),
                ]));
                lines.push(Line::from(""));
            }

            OutputKind::Info => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "◆ ".to_string(),
                        Style::default()
                            .fg(Color::Rgb(120, 170, 255))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        block.content.clone(),
                        Style::default().fg(Color::Rgb(150, 190, 240)),
                    ),
                ]));
                lines.push(Line::from(""));
            }
        }
    }

    lines
}
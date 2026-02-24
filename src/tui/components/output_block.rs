use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::tui::app::{OutputBlock, OutputKind};

pub fn render_lines(blocks: &[OutputBlock]) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for block in blocks {
        let content_lines: Vec<&str> = block.content.lines().collect();

        match block.kind {
            OutputKind::Command => {
                render_multiline(
                    &mut lines,
                    &content_lines,
                    "> ",
                    Style::default()
                        .fg(Color::Rgb(160, 120, 220))
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(Color::Rgb(200, 200, 210))
                        .add_modifier(Modifier::BOLD),
                );
            }

            OutputKind::Success => {
                render_multiline(
                    &mut lines,
                    &content_lines,
                    "✔ ",
                    Style::default()
                        .fg(Color::Rgb(100, 220, 120))
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Rgb(160, 230, 170)),
                );
            }

            OutputKind::Error => {
                render_multiline(
                    &mut lines,
                    &content_lines,
                    "✖ ",
                    Style::default()
                        .fg(Color::Rgb(220, 80, 80))
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Rgb(230, 100, 100)),
                );
            }

            OutputKind::Info => {
                render_multiline(
                    &mut lines,
                    &content_lines,
                    "◆ ",
                    Style::default()
                        .fg(Color::Rgb(120, 170, 255))
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Rgb(150, 190, 240)),
                );
            }
        }

        // Spacer between output blocks
        lines.push(Line::from(""));
    }

    lines
}

/// Renders multi-line content with a styled prefix only on the first line.
/// Subsequent lines are properly indented.
fn render_multiline(
    lines: &mut Vec<Line<'static>>,
    content_lines: &[&str],
    prefix: &str,
    prefix_style: Style,
    content_style: Style,
) {
    if content_lines.is_empty() {
        return;
    }

    // First line with prefix
    lines.push(Line::from(vec![
        Span::styled(prefix.to_string(), prefix_style),
        Span::styled(content_lines[0].to_string(), content_style),
    ]));

    // Subsequent lines aligned with content (indent same width as prefix)
    let indent = " ".repeat(prefix.len());

    for line in content_lines.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::raw(indent.clone()),
            Span::styled(line.to_string(), content_style),
        ]));
    }
}
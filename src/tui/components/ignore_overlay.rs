use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, List, ListItem},
};

// Use the correct struct name from your updated app.rs
use crate::tui::app::IgnoreOverlay;

pub fn draw(f: &mut Frame, ov: &IgnoreOverlay) {
    let area = f.area();
    let popup = centered_rect(75, 65, area);

    f.render_widget(Clear, popup);

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(100, 160, 255)))
        .style(Style::default().bg(Color::Rgb(12, 12, 18)))
        .title(Span::styled(
            "  🛡️  Manage Ignored Findings  ",
            Style::default().add_modifier(Modifier::BOLD),
        ));

    let inner = outer_block.inner(popup);
    f.render_widget(outer_block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // List of items
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Clear all option
            Constraint::Length(1), // Hint
        ])
        .split(inner);

    // Render Items - Use ov.selected to match app.rs
    let items: Vec<ListItem> = ov.items.iter().enumerate().map(|(i, item)| {
        let is_selected = i == ov.selected;

        let style = if is_selected {
            Style::default().bg(Color::Rgb(30, 30, 45)).fg(Color::White)
        } else {
            Style::default().fg(Color::Rgb(140, 140, 160))
        };

        let prefix = if is_selected { "> " } else { "  " };

        let content = Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Rgb(220, 80, 80))),
            Span::styled(format!("[{}] ", item.short_id), Style::default().fg(Color::Rgb(160, 120, 255))),
            Span::styled(item.variable.clone(), style),
            Span::styled(format!(" ({})", item.source), Style::default().fg(Color::Rgb(80, 80, 100))),
        ]);

        ListItem::new(content)
    }).collect();

    if items.is_empty() {
        f.render_widget(
            Paragraph::new("No ignored findings found.").alignment(Alignment::Center),
            chunks[0]
        );
    } else {
        f.render_widget(List::new(items), chunks[0]);
    }

    // Clear All "Button" - Logic matching handle_ignore_key
    let is_clear_selected = ov.is_clear_all_selected();
    let clear_all_style = if is_clear_selected {
        Style::default().fg(Color::Rgb(220, 80, 80)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(100, 100, 120))
    };

    let clear_line = Line::from(vec![
        Span::styled(if is_clear_selected { "> " } else { "  " }, clear_all_style),
        Span::styled("Clear ALL ignored findings", clear_all_style)
    ]);

    f.render_widget(
        Paragraph::new(clear_line)
            .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::Rgb(30, 30, 40)))),
        chunks[2]
    );

    // Hint
    let hint = Line::from(vec![
        Span::styled("  ↑/↓ ", Style::default().fg(Color::Rgb(160, 120, 255))),
        Span::styled("navigate  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("Enter ", Style::default().fg(Color::Rgb(160, 120, 255))),
        Span::styled("remove  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("Esc ", Style::default().fg(Color::Rgb(160, 120, 255))),
        Span::styled("exit", Style::default().fg(Color::Rgb(100, 100, 120))),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[3]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(layout[1])[1]
}
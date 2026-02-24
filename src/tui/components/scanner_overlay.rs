use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::tui::app::{ScanChoice, ScannerOverlay};

pub fn draw(f: &mut Frame, ov: &ScannerOverlay) {
    let area = f.area();
    let popup = centered_rect(92, 72, area);

    f.render_widget(Clear, popup);

    let finding = match ov.current() {
        Some(f) => f,
        None => return,
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(70, 80, 100)))
        .style(Style::default().bg(Color::Rgb(15, 17, 22)))
        .title(Span::styled(
            format!("  🔍 Secret Scanner — Finding {}/{}  ", ov.current_index + 1, ov.total()),
            Style::default().fg(Color::Rgb(130, 150, 180)).add_modifier(Modifier::BOLD),
        ));

    // Safe inner area calculation
    let inner = outer.inner(popup);
    f.render_widget(outer, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), Constraint::Length(1), Constraint::Length(1),
            Constraint::Length(1), Constraint::Length(1), Constraint::Length(1),
            Constraint::Length(1), Constraint::Length(1), Constraint::Min(3),
            Constraint::Length(1), Constraint::Length(3), Constraint::Length(1),
        ])
        .split(inner);

    // --- File Path ---
    let abs_path = std::env::current_dir()
        .map(|p| p.join(&finding.file).to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| finding.file.replace('\\', "/"));

    let path_line = Line::from(vec![Span::styled(
        format!("file:///{}", format!("{}:{}:{}", abs_path, finding.line, finding.column)),
        Style::default().fg(Color::Rgb(130, 150, 180)).add_modifier(Modifier::UNDERLINED),
    )]);
    f.render_widget(Paragraph::new(path_line), chunks[1]);

    // --- Code Block ---
    let bar = Span::styled("    │", Style::default().fg(Color::Rgb(65, 70, 85)));
    f.render_widget(Paragraph::new(Line::from(bar.clone())), chunks[3]);
    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled(format!(" {:>4} ", finding.line), Style::default().fg(Color::Rgb(100, 100, 110))),
        Span::styled(finding.content.trim().to_string(), Style::default().fg(Color::Rgb(200, 180, 130))),
    ])), chunks[4]);
    f.render_widget(Paragraph::new(Line::from(bar)), chunks[5]);

    // --- Detection ---
    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled("    = detected: ", Style::default().fg(Color::Rgb(110, 115, 130))),
        Span::styled(finding.secret_type.clone(), Style::default().fg(Color::Rgb(180, 130, 110)).add_modifier(Modifier::BOLD)),
    ])), chunks[6]);

    // --- Buttons ---
    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(32), Constraint::Length(4), Constraint::Length(32), Constraint::Min(1)])
        .split(chunks[10]);

    let (ignore_s, ignore_b, keep_s, keep_b) = if ov.choice == ScanChoice::Ignore {
        (Style::default().fg(Color::Rgb(15, 17, 22)).bg(Color::Rgb(180, 100, 100)).add_modifier(Modifier::BOLD), Style::default().fg(Color::Rgb(180, 100, 100)),
         Style::default().fg(Color::Rgb(110, 115, 130)), Style::default().fg(Color::Rgb(55, 60, 75)))
    } else {
        (Style::default().fg(Color::Rgb(110, 115, 130)), Style::default().fg(Color::Rgb(55, 60, 75)),
         Style::default().fg(Color::Rgb(15, 17, 22)).bg(Color::Rgb(120, 160, 130)).add_modifier(Modifier::BOLD), Style::default().fg(Color::Rgb(120, 160, 130)))
    };

    f.render_widget(Paragraph::new("  ✖  Ignore permanently  ").style(ignore_s).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(ignore_b)).alignment(Alignment::Center), btn_chunks[1]);
    f.render_widget(Paragraph::new("  ✔  Keep showing  ").style(keep_s).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(keep_b)).alignment(Alignment::Center), btn_chunks[3]);

    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled("  ←/→ ", Style::default().fg(Color::Rgb(130, 150, 180))),
        Span::styled("select  ", Style::default().fg(Color::Rgb(200, 200, 210))),
        Span::styled("Enter ", Style::default().fg(Color::Rgb(130, 150, 180))),
        Span::styled("confirm", Style::default().fg(Color::Rgb(200, 200, 210))),
    ])), chunks[11]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let v = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).split(v[1])[1]
}
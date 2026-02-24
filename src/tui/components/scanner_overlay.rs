use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::{ScanChoice, ScannerOverlay};

pub fn draw(f: &mut Frame, ov: &ScannerOverlay) {
    let area = f.area();

    // Centered overlay: 90% wide, ~70% tall
    let popup = centered_rect(92, 72, area);

    // Clear background
    f.render_widget(Clear, popup);

    let finding = match ov.current() {
        Some(f) => f,
        None => return,
    };

    // Outer border
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(220, 80, 80)))
        .style(Style::default().bg(Color::Rgb(12, 12, 18)))
        .title(Span::styled(
            format!(
                "  🔍 Secret Scanner — Finding {}/{} ",
                ov.current_index + 1,
                ov.total()
            ),
            Style::default()
                .fg(Color::Rgb(220, 80, 80))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left);

    f.render_widget(outer, popup);

    // Inner area (inset from border)
    let inner = Rect {
        x: popup.x + 2,
        y: popup.y + 1,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(2),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // file path
            Constraint::Length(1),  // spacer
            Constraint::Length(1),  // separator |
            Constraint::Length(1),  // code line
            Constraint::Length(1),  // separator |
            Constraint::Length(1),  // = detected
            Constraint::Length(1),  // spacer
            Constraint::Min(3),     // context / description block
            Constraint::Length(1),  // spacer
            Constraint::Length(3),  // choice buttons
            Constraint::Length(1),  // hint
        ])
        .split(inner);

    // ── File path (OSC 8 hyperlink via ANSI) ──────────────────────────────────
    // We embed the OSC 8 escape sequence directly so clicking opens the file.
    let file_path = &finding.file;
    let line_num = finding.line;

    // Build clickable path span using OSC 8
    // \x1b]8;;file://path\x1b\\text\x1b]8;;\x1b\\
    let abs_path = std::env::current_dir()
        .map(|p| p.join(file_path).to_string_lossy().to_string())
        .unwrap_or_else(|_| file_path.clone());

    let hyperlink = format!(
        "\x1b]8;;file://{}\x1b\\{}:{}\x1b]8;;\x1b\\",
        abs_path, file_path, line_num
    );

    let path_line = Line::from(vec![
        Span::styled(
            hyperlink,
            Style::default()
                .fg(Color::Rgb(100, 160, 255))
                .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
        ),
        Span::styled(
            format!(":{}:{}", line_num, finding.column),
            Style::default().fg(Color::Rgb(140, 140, 160)),
        ),
    ]);
    f.render_widget(Paragraph::new(path_line), chunks[1]);

    // ── Code block ────────────────────────────────────────────────────────────
    let bar = Span::styled("    │", Style::default().fg(Color::Rgb(70, 70, 85)));
    let line_num_span = Span::styled(
        format!(" {:>4} ", line_num),
        Style::default().fg(Color::Rgb(100, 100, 120)),
    );
    let code_span = Span::styled(
        finding.content.trim().to_string(),
        Style::default().fg(Color::Rgb(240, 200, 100)),
    );

    f.render_widget(Paragraph::new(Line::from(bar.clone())), chunks[3]);
    f.render_widget(
        Paragraph::new(Line::from(vec![line_num_span, code_span])),
        chunks[4],
    );
    f.render_widget(Paragraph::new(Line::from(bar)), chunks[5]);

    // ── Detection label ───────────────────────────────────────────────────────
    let detected = Line::from(vec![
        Span::styled(
            "    = detected: ",
            Style::default().fg(Color::Rgb(120, 120, 140)),
        ),
        Span::styled(
            finding.secret_type.clone(),
            Style::default()
                .fg(Color::Rgb(255, 130, 80))
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(detected), chunks[6]);

    // ── Fingerprint info ──────────────────────────────────────────────────────
    if let Some(commit) = &finding.commit {
        let info = Line::from(vec![
            Span::styled("    @ commit ", Style::default().fg(Color::Rgb(100, 100, 120))),
            Span::styled(
                &commit[..8.min(commit.len())],
                Style::default().fg(Color::Rgb(160, 120, 255)),
            ),
        ]);
        f.render_widget(Paragraph::new(info), chunks[8]);
    }

    // ── Choice buttons ────────────────────────────────────────────────────────
    let btn_area = chunks[10];
    let btn_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),          // spacer left
            Constraint::Length(32),      // ignore button
            Constraint::Length(4),       // gap
            Constraint::Length(32),      // keep button
            Constraint::Min(1),          // spacer right
        ])
        .split(btn_area);

    let (ignore_style, ignore_border, keep_style, keep_border) = match ov.choice {
        ScanChoice::Ignore => (
            Style::default().fg(Color::Rgb(10, 10, 14)).bg(Color::Rgb(220, 80, 80)).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(220, 80, 80)),
            Style::default().fg(Color::Rgb(140, 140, 160)),
            Style::default().fg(Color::Rgb(50, 50, 65)),
        ),
        ScanChoice::Keep => (
            Style::default().fg(Color::Rgb(140, 140, 160)),
            Style::default().fg(Color::Rgb(50, 50, 65)),
            Style::default().fg(Color::Rgb(10, 10, 14)).bg(Color::Rgb(80, 200, 120)).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(80, 200, 120)),
        ),
    };

    let ignore_btn = Paragraph::new(Line::from(vec![
        Span::styled("  ✖  Ignore permanently  ", ignore_style),
    ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(ignore_border),
        )
        .alignment(Alignment::Center);

    let keep_btn = Paragraph::new(Line::from(vec![
        Span::styled("  ✔  Keep showing  ", keep_style),
    ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(keep_border),
        )
        .alignment(Alignment::Center);

    f.render_widget(ignore_btn, btn_chunks[1]);
    f.render_widget(keep_btn, btn_chunks[3]);

    // ── Hint bar ──────────────────────────────────────────────────────────────
    let hint = Line::from(vec![
        Span::styled("  ←/→ ", Style::default().fg(Color::Rgb(160, 120, 255))),
        Span::styled("select  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("Enter ", Style::default().fg(Color::Rgb(160, 120, 255))),
        Span::styled("confirm  ", Style::default().fg(Color::Rgb(100, 100, 120))),
        Span::styled("Esc/q ", Style::default().fg(Color::Rgb(160, 120, 255))),
        Span::styled("skip all", Style::default().fg(Color::Rgb(100, 100, 120))),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[11]);
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
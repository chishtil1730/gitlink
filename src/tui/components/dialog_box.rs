use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn render(input: &str, cursor_pos: usize, _is_executing: bool) -> Paragraph<'static> {
    let prompt = Span::styled(
        "> ",
        Style::default()
            .fg(Color::Rgb(160, 120, 220))
            .add_modifier(Modifier::BOLD),
    );

    let prefix: String = input[..cursor_pos].to_string();
    let cursor_ch: String = input
        .chars()
        .nth(cursor_pos)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let suffix: String = if cursor_pos < input.len() {
        input[cursor_pos + cursor_ch.len()..].to_string()
    } else {
        String::new()
    };

    // Plain white text — real cursor is positioned by f.set_cursor_position() in ui.rs
    let before = Span::styled(prefix, Style::default().fg(Color::White));
    let cursor = Span::styled(cursor_ch, Style::default().fg(Color::White));
    let after = Span::styled(suffix, Style::default().fg(Color::White));

    let placeholder = Span::styled(
        "Type your message or /command",
        Style::default().fg(Color::Rgb(80, 80, 90)),
    );

    let content = if input.is_empty() {
        Line::from(vec![prompt, placeholder])
    } else {
        Line::from(vec![prompt, before, cursor, after])
    };

    Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(55, 55, 65)))
            .style(Style::default().bg(Color::Rgb(18, 18, 24))),
    )
}

pub fn render_with_spinner(
    input: &str,
    cursor_pos: usize,
    is_executing: bool,
    _elapsed: f32,
) -> Paragraph<'static> {
    render(input, cursor_pos, is_executing)
}

/// Renders the standalone spinner bar shown above the dialog box during execution.
pub fn render_spinner_bar(spin_elapsed: f32) -> Paragraph<'static> {
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    // 12.5 fps → one frame every 80 ms, starts at frame 0 for every new command
    let frame = spinner_frames[(spin_elapsed * 12.5) as usize % spinner_frames.len()];

    Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", frame),
            Style::default()
                .fg(Color::Rgb(0, 220, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Executing",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "...",
            Style::default().fg(Color::Rgb(200, 200, 200)),
        ),
    ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(0, 160, 80)))
                .style(Style::default().bg(Color::Rgb(10, 10, 14))),
        )
}
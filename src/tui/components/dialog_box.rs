use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

pub fn render(input: &str, cursor_pos: usize, is_executing: bool) -> Paragraph<'static> {
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

    let before = Span::styled(prefix, Style::default().fg(Color::White));
    let cursor = Span::styled(
        cursor_ch,
        Style::default()
            .fg(Color::Black)
            .bg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let after = Span::styled(suffix, Style::default().fg(Color::White));

    let placeholder = Span::styled(
        "Type your message or /command",
        Style::default().fg(Color::Rgb(80, 80, 90)),
    );

    let content = if input.is_empty() && !is_executing {
        Line::from(vec![prompt, placeholder])
    } else if is_executing {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        // Use a static counter trick via the input string being empty during execution
        let frame = spinner_frames[0]; // will be updated in ui.rs with elapsed
        Line::from(vec![
            prompt,
            Span::styled(
                format!("{} Executing...", frame),
                Style::default().fg(Color::Rgb(120, 180, 255)),
            ),
        ])
    } else {
        Line::from(vec![prompt, before, cursor, after])
    };

    let border_color = if is_executing {
        Color::Rgb(120, 180, 255)
    } else {
        Color::Rgb(55, 55, 65)
    };

    Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(Color::Rgb(18, 18, 24))),
    )
}

pub fn render_with_spinner(
    input: &str,
    cursor_pos: usize,
    is_executing: bool,
    elapsed: f32,
) -> Paragraph<'static> {
    let prompt = Span::styled(
        "> ",
        Style::default()
            .fg(Color::Rgb(160, 120, 220))
            .add_modifier(Modifier::BOLD),
    );

    if is_executing {
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = (elapsed * 10.0) as usize % spinner_frames.len();
        let frame = spinner_frames[frame_idx];

        return Paragraph::new(Line::from(vec![
            prompt,
            Span::styled(
                format!("{} Executing...", frame),
                Style::default().fg(Color::Rgb(120, 180, 255)),
            ),
        ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(120, 180, 255)))
                    .style(Style::default().bg(Color::Rgb(18, 18, 24))),
            );
    }

    render(input, cursor_pos, is_executing)
}
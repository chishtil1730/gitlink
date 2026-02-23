use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Paragraph},
};

use super::{
    app::App,
    components::{dialog_box, logo, output_block, suggestion_list},
};

const MAX_SUGGESTIONS_SHOWN: usize = 8;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(10, 10, 14))),
        area,
    );

    let logo_h = logo::logo_height();
    let dialog_h = 3u16;
    let suggestion_h = if app.show_suggestions {
        (app.filtered_commands.len().min(MAX_SUGGESTIONS_SHOWN) + 2) as u16
    } else {
        0
    };

    // Total fixed bottom area (dialog + suggestions)
    let fixed_bottom = dialog_h + suggestion_h;

    // Output area sits between logo and dialog
    let output_h = area.height.saturating_sub(logo_h + fixed_bottom);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(logo_h),
            Constraint::Length(output_h),
            Constraint::Length(dialog_h),
            Constraint::Length(suggestion_h),
        ])
        .split(area);

    // --- Logo ---
    f.render_widget(logo::render(app.elapsed), chunks[0]);

    // --- Output area ---
    let output_lines = output_block::render_lines(&app.outputs);
    let total_lines = output_lines.len() as u16;
    let visible_h = chunks[1].height.saturating_sub(0);

    // Compute scroll: if scroll is 0, pin to bottom
    let max_scroll = total_lines.saturating_sub(visible_h);
    let scroll = if app.output_scroll == 0 {
        max_scroll
    } else {
        max_scroll.saturating_sub(app.output_scroll)
    };

    // Pad lines with empty lines at top to push content to bottom
    let mut padded: Vec<Line> = Vec::new();
    if total_lines < visible_h {
        for _ in 0..(visible_h - total_lines) {
            padded.push(Line::from(""));
        }
    }
    padded.extend(output_lines);

    let output_widget = Paragraph::new(padded)
        .scroll((scroll, 0))
        .style(Style::default().bg(Color::Rgb(10, 10, 14)));

    f.render_widget(output_widget, chunks[1]);

    // --- Dialog box ---
    let dialog = dialog_box::render_with_spinner(
        &app.input,
        app.cursor_pos,
        app.is_executing,
        app.elapsed,
    );
    f.render_widget(dialog, chunks[2]);

    // --- Suggestion list ---
    if app.show_suggestions && !app.filtered_commands.is_empty() {
        let suggestions = suggestion_list::render(
            &app.filtered_commands,
            app.selected_index,
            MAX_SUGGESTIONS_SHOWN,
        );
        f.render_widget(suggestions, chunks[3]);
    }
}
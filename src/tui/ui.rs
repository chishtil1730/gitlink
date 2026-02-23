use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
};

use super::{
    app::App,
    components::{dialog_box, logo, output_block, suggestion_list},
};

const MAX_SUGGESTIONS_SHOWN: usize = 8;

/// Padding rows above the logo. Change this to move the logo down.
const LOGO_TOP_PADDING: u16 = 2;

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(10, 10, 14))),
        area,
    );

    let logo_h = logo::logo_height();
    let dialog_h = 3u16;
    let suggestion_h = if app.show_suggestions && !app.filtered_commands.is_empty() {
        (app.filtered_commands.len().min(MAX_SUGGESTIONS_SHOWN) + 2) as u16
    } else {
        0
    };

    // Total fixed bottom area (dialog + suggestions)
    let fixed_bottom = dialog_h + suggestion_h;

    // Output area: everything between logo block and dialog
    let logo_block_h = LOGO_TOP_PADDING + logo_h;
    let output_h = area.height.saturating_sub(logo_block_h + fixed_bottom);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(LOGO_TOP_PADDING), // top padding above logo
            Constraint::Length(logo_h),           // logo
            Constraint::Length(output_h),         // scrollable output
            Constraint::Length(dialog_h),         // dialog box
            Constraint::Length(suggestion_h),     // suggestions (0 when hidden)
        ])
        .split(area);

    // --- Logo ---
    f.render_widget(logo::render(app.elapsed), chunks[1]);

    // --- Output area ---
    // output_scroll == 0 means "follow bottom". Scrolling UP increases output_scroll.
    let output_lines = output_block::render_lines(&app.outputs);
    let total_lines = output_lines.len() as u16;
    let visible_h = chunks[2].height;

    // How far from the bottom the user has scrolled (0 = pinned to bottom)
    let clamped_scroll = app.output_scroll.min(total_lines.saturating_sub(visible_h));
    // ratatui scroll(row, col): row 0 = top of content
    // We want bottom-pinned by default, so scroll row = max_scroll - user_scroll
    let max_scroll = total_lines.saturating_sub(visible_h);
    let scroll_row = max_scroll.saturating_sub(clamped_scroll);

    let output_widget = Paragraph::new(output_lines)
        .scroll((scroll_row, 0))
        .style(Style::default().bg(Color::Rgb(10, 10, 14)));

    f.render_widget(output_widget, chunks[2]);

    // --- Dialog box ---
    let dialog = dialog_box::render_with_spinner(
        &app.input,
        app.cursor_pos,
        app.is_executing,
        app.elapsed,
    );
    f.render_widget(dialog, chunks[3]);

    // --- Suggestion list ---
    if app.show_suggestions && !app.filtered_commands.is_empty() {
        let suggestions = suggestion_list::render(
            &app.filtered_commands,
            app.selected_index,
            MAX_SUGGESTIONS_SHOWN,
        );
        f.render_widget(suggestions, chunks[4]);
    }
}
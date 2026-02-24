use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph, Clear},
};

use super::{
    app::{App, Overlay},
    components::{
        dialog_box,
        logo,
        output_block,
        planner_overlay,
        scanner_overlay,
        suggestion_list,
        ignore_overlay,
    },
};

const MAX_SUGGESTIONS_SHOWN: usize = 8;
const LOGO_TOP_PADDING: u16 = 2;

pub fn draw(f: &mut Frame, app: &App, spin_elapsed: f32) {
    let area = f.area();

    // 1️⃣ FULL FRAME CLEAR (CRITICAL FIX)
    f.render_widget(Clear, area);

    // 2️⃣ Background Fill
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(10, 10, 14))),
        area,
    );

    let logo_h = logo::logo_height();
    let dialog_h = 3u16;
    let spinner_h = if app.is_executing { 3u16 } else { 0u16 };
    let suggestion_h = if app.show_suggestions && !app.filtered_commands.is_empty() {
        (app.filtered_commands.len().min(MAX_SUGGESTIONS_SHOWN) + 2) as u16
    } else {
        0
    };

    // 3️⃣ Layout — spinner slot sits between output and dialog
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(LOGO_TOP_PADDING),
            Constraint::Length(logo_h),
            Constraint::Min(0),          // output area absorbs remaining space
            Constraint::Length(spinner_h),
            Constraint::Length(dialog_h),
            Constraint::Length(suggestion_h),
        ])
        .split(area);

    // --- Logo ---
    let time_param = if app.output_scroll >= 20.0 {
        app.elapsed * 1.5
    } else {
        app.elapsed
    };
    f.render_widget(logo::render(time_param), chunks[1]);

    // --- Output area ---
    let output_lines = output_block::render_lines(&app.outputs);
    let total_lines = output_lines.len() as u16;
    let visible_h = chunks[2].height;
    let max_scroll = total_lines.saturating_sub(visible_h);
    let current_scroll_u16 = app.output_scroll as u16;
    let clamped = current_scroll_u16.min(max_scroll);
    let scroll_row = max_scroll.saturating_sub(clamped);

    f.render_widget(
        Paragraph::new(output_lines)
            .scroll((scroll_row, 0))
            .style(Style::default().bg(Color::Rgb(10, 10, 14))),
        chunks[2],
    );

    // --- Spinner bar (above dialog, only while executing) ---
    if app.is_executing {
        f.render_widget(dialog_box::render_spinner_bar(spin_elapsed), chunks[3]);
    }

    // --- Dialog box ---
    let dialog_area = chunks[4];
    f.render_widget(
        dialog_box::render_with_spinner(&app.input, app.cursor_pos, app.is_executing, app.elapsed),
        dialog_area,
    );

    // Position the real terminal cursor inside the dialog box.
    if !app.is_executing {
        let cx = dialog_area.x + 1 + 2 + app.cursor_pos as u16;
        let cy = dialog_area.y + 1;
        f.set_cursor_position((cx, cy));
    }

    // --- Suggestion list ---
    if app.show_suggestions && !app.filtered_commands.is_empty() {
        f.render_widget(
            suggestion_list::render(&app.filtered_commands, app.selected_index, MAX_SUGGESTIONS_SHOWN),
            chunks[5],
        );
    }

    // --- Overlays (Drawn LAST to stay on top) ---
    match &app.overlay {
        Some(Overlay::Scanner(ov)) => { scanner_overlay::draw(f, ov); }
        Some(Overlay::Planner(ov)) => { planner_overlay::draw(f, ov); }
        Some(Overlay::Ignore(ov)) => { ignore_overlay::draw(f, ov); }
        None => {}
    }
}
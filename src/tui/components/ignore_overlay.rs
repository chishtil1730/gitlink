use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear,
        List, ListItem, ListState, Paragraph,
    },
};

use crate::tui::app::IgnoreOverlay;
use crate::scanner::ignore::{load_ignore_db, save_ignore_db, remove_by_short_id};

impl IgnoreOverlay {
    pub fn next(&mut self) {
        let total = self.items.len() + 2;
        if total > 0 { self.selected = (self.selected + 1) % total; }
    }

    pub fn previous(&mut self) {
        let total = self.items.len() + 2;
        if total > 0 {
            if self.selected == 0 { self.selected = total - 1; }
            else { self.selected -= 1; }
        }
    }

    pub fn confirm_selection(&mut self) {
        let len = self.items.len();
        if self.selected < len {
            let removed = self.items.remove(self.selected);
            remove_by_short_id(&removed.short_id);
            if self.selected >= self.items.len() && !self.items.is_empty() {
                self.selected = self.items.len() - 1;
            }
        } else if self.selected == len {
            self.items.clear();
            let mut db = load_ignore_db();
            db.ignored.clear();
            save_ignore_db(&db);
            self.selected = 0;
        } else {
            self.done = true;
        }
    }
}

pub fn draw(f: &mut Frame, ov: &IgnoreOverlay) {
    let area = f.area();

    // Use fixed-margin clamped rect instead of percentage-based
    let popup = safe_centered_rect_clamped(area);

    // 1. Clear behind the popup
    f.render_widget(Clear, popup);

    // 2. Flood-fill background to prevent terminal bleed-through
    let bg_block = Block::default()
        .style(Style::default().bg(Color::Rgb(15, 17, 22)));
    f.render_widget(bg_block, popup);

    // 3. Outer bordered block
    let outer_block = Block::default()
        .title(Span::styled(
            " 🛡️  Manage Ignored Findings ",
            Style::default()
                .fg(Color::Rgb(130, 150, 180))
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(70, 80, 100)))
        .style(Style::default().bg(Color::Rgb(15, 17, 22)));

    // Compute inner area BEFORE rendering the block
    let inner = outer_block.inner(popup);
    f.render_widget(outer_block, popup);

    // 4. Small horizontal padding inside the border
    let padded = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };

    // 5. Split: scrollable list on top, footer pinned to bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(padded);

    // 6. Build list items
    let mut items: Vec<ListItem> = ov.items.iter().map(|item| {
        let source_info = if item.source == "history" {
            item.commit
                .as_ref()
                .map(|c| format!("(commit {})", &c[..8.min(c.len())]))
                .unwrap_or_else(|| "(history)".to_string())
        } else {
            "(working)".to_string()
        };

        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{:<8} ", item.short_id),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:<20} ", item.variable)),
            Span::styled(
                source_info,
                Style::default().fg(Color::Rgb(110, 115, 130)),
            ),
        ]))
    }).collect();

    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            "🗑️  Clear ALL ignored",
            Style::default()
                .fg(Color::Rgb(180, 100, 100))
                .add_modifier(Modifier::BOLD),
        ),
    ])));

    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            "🚪 Exit",
            Style::default().fg(Color::Rgb(130, 150, 180)),
        ),
    ])));

    let mut state = ListState::default();
    state.select(Some(ov.selected));

    // 7. Render list
    let list = List::new(items)
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 35, 45))
                .fg(Color::Rgb(200, 180, 130)),
        );

    f.render_stateful_widget(list, chunks[0], &mut state);

    // 8. Footer hint bar
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Enter ", Style::default().fg(Color::Rgb(130, 150, 180))),
            Span::styled("select  ", Style::default().fg(Color::Rgb(200, 200, 210))),
            Span::styled(" Esc ", Style::default().fg(Color::Rgb(130, 150, 180))),
            Span::styled("close", Style::default().fg(Color::Rgb(200, 200, 210))),
        ]))
            .alignment(Alignment::Center),
        chunks[1],
    );
}

/// Computes a centered popup rect using fixed cell margins instead of percentages.
/// This prevents the popup from being clipped when the terminal is small,
/// since percentage-based sizing can exceed the available area on narrow terminals.
///
/// Guarantees at least `h_margin` cols on each side and `v_margin` rows top/bottom.
/// Also caps width/height so it doesn't grow absurdly large on big terminals.
fn safe_centered_rect_clamped(r: Rect) -> Rect {
    let h_margin: u16 = 4;
    let v_margin: u16 = 3;

    // Available space after applying margins
    let max_width  = r.width.saturating_sub(h_margin * 2);
    let max_height = r.height.saturating_sub(v_margin * 2);

    // Cap to a comfortable reading size on large terminals
    let width  = max_width.min(120);
    let height = max_height.min(30);

    // Center within the full area
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;

    Rect { x, y, width, height }
}
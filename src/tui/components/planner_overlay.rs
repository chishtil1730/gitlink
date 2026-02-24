use chrono::Local;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::{InputField, PlannerFocus, PlannerMode, PlannerOverlay, planner_scratch_peek};

pub fn draw(f: &mut Frame, ov: &PlannerOverlay) {
    let area = f.area();
    let popup = centered_rect(94, 88, area);

    f.render_widget(Clear, popup);

    // ── Outer shell ───────────────────────────────────────────────────────────
    let total = ov.tasks.len();
    let done = ov.tasks.iter().filter(|t| t.completed).count();

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(100, 80, 200)))
        .style(Style::default().bg(Color::Rgb(10, 10, 16)))
        .title(Span::styled(
            format!("  📋 GitLink Planner  ·  {}/{} done  ", done, total),
            Style::default()
                .fg(Color::Rgb(160, 130, 255))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(
            "  Tab: switch panel  ·  a: add  ·  e: edit  ·  d: delete  ·  Space: toggle  ·  u/r: undo/redo  ·  q: close  ",
            Style::default().fg(Color::Rgb(70, 70, 90)),
        ));

    f.render_widget(outer, popup);

    let inner = Rect {
        x: popup.x + 1,
        y: popup.y + 1,
        width: popup.width.saturating_sub(2),
        height: popup.height.saturating_sub(2),
    };

    // ── Two panels ────────────────────────────────────────────────────────────
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(inner);

    draw_list_panel(f, ov, panels[0]);
    draw_detail_panel(f, ov, panels[1]);

    // ── Input modal ───────────────────────────────────────────────────────────
    match ov.mode {
        PlannerMode::AddingTask => {
            draw_input_modal(f, ov, popup, "  ✚ Add New Task  ");
        }
        PlannerMode::EditingTask => {
            draw_input_modal(f, ov, popup, "  ✎ Edit Task  ");
        }
        PlannerMode::ConfirmDelete => {
            draw_confirm_modal(f, ov, popup);
        }
        _ => {}
    }
}

fn draw_list_panel(f: &mut Frame, ov: &PlannerOverlay, area: Rect) {
    let is_focused = ov.focus == PlannerFocus::List;
    let border_color = if is_focused {
        Color::Rgb(160, 130, 255)
    } else {
        Color::Rgb(50, 50, 65)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Rgb(10, 10, 16)))
        .title(Span::styled(
            "  Tasks  ",
            Style::default().fg(border_color).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(block, area);

    let list_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    if ov.tasks.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No tasks yet.",
                Style::default().fg(Color::Rgb(70, 70, 90)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'a' to add one.",
                Style::default().fg(Color::Rgb(100, 80, 200)),
            )),
        ])
            .alignment(Alignment::Center);
        f.render_widget(empty, list_area);
        return;
    }

    let visible_h = list_area.height as usize;
    let start = ov.scroll;
    let end = (start + visible_h).min(ov.tasks.len());

    let lines: Vec<Line> = ov.tasks[start..end]
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let real_idx = start + i;
            let is_sel = real_idx == ov.selected;

            let pointer = if is_sel && ov.focus == PlannerFocus::List {
                Span::styled("▶ ", Style::default().fg(Color::Rgb(160, 130, 255)))
            } else {
                Span::raw("  ")
            };

            let checkbox = if task.completed {
                Span::styled("✔ ", Style::default().fg(Color::Rgb(80, 200, 120)))
            } else {
                Span::styled("○ ", Style::default().fg(Color::Rgb(100, 100, 130)))
            };

            let title_style = if task.completed {
                Style::default()
                    .fg(Color::Rgb(80, 80, 100))
                    .add_modifier(Modifier::CROSSED_OUT)
            } else if is_sel {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 200, 215))
            };

            let bg = if is_sel && ov.focus == PlannerFocus::List {
                Style::default().bg(Color::Rgb(30, 25, 50))
            } else {
                Style::default()
            };

            // Truncate title to fit
            let max_title = (list_area.width as usize).saturating_sub(6);
            let title_text = if task.title.len() > max_title {
                format!("{}…", &task.title[..max_title.saturating_sub(1)])
            } else {
                task.title.clone()
            };

            Line::styled(
                format!(""),
                bg,
            );

            // Build tag badges
            let tags_span = if !task.tags.is_empty() {
                let tag_str = task.tags
                    .iter()
                    .map(|t| format!(" {} ", t))
                    .collect::<Vec<_>>()
                    .join(" ");
                Span::styled(
                    format!("  {}", tag_str),
                    Style::default().fg(Color::Rgb(120, 100, 200)),
                )
            } else {
                Span::raw("")
            };

            Line::from(vec![
                pointer,
                checkbox,
                Span::styled(title_text, title_style),
                tags_span,
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), list_area);

    // Scrollbar hint
    if ov.tasks.len() > visible_h {
        let scroll_pct = if ov.tasks.len() > 1 {
            (ov.selected * 100) / (ov.tasks.len() - 1)
        } else { 0 };

        let hint = Paragraph::new(Line::from(Span::styled(
            format!(" {}/{} ", ov.selected + 1, ov.tasks.len()),
            Style::default().fg(Color::Rgb(70, 70, 90)),
        )))
            .alignment(Alignment::Right);

        let hint_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };
        f.render_widget(hint, hint_area);
    }
}

fn draw_detail_panel(f: &mut Frame, ov: &PlannerOverlay, area: Rect) {
    let is_focused = ov.focus == PlannerFocus::Detail;
    let border_color = if is_focused {
        Color::Rgb(160, 130, 255)
    } else {
        Color::Rgb(50, 50, 65)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Rgb(12, 12, 18)))
        .title(Span::styled(
            "  Detail  ",
            Style::default().fg(border_color).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(block, area);

    let detail_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(3),
    };

    let task = match ov.selected_task() {
        Some(t) => t,
        None => {
            let empty = Paragraph::new(Span::styled(
                "Select a task to view details",
                Style::default().fg(Color::Rgb(70, 70, 90)),
            ))
                .alignment(Alignment::Center);
            f.render_widget(empty, detail_area);
            return;
        }
    };

    let status_str = if task.completed { "✔ Completed" } else { "○ Pending" };
    let status_color = if task.completed {
        Color::Rgb(80, 200, 120)
    } else {
        Color::Rgb(160, 130, 255)
    };

    let created = task.created_at.with_timezone(&Local);
    let updated = task.updated_at.with_timezone(&Local);

    let mut lines: Vec<Line> = vec![
        // Title
        Line::from(vec![
            Span::styled(
                task.title.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        // Status badge
        Line::from(vec![
            Span::styled("Status   ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled(
                status_str,
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        // Dates
        Line::from(vec![
            Span::styled("Created  ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled(
                created.format("%Y-%m-%d  %H:%M").to_string(),
                Style::default().fg(Color::Rgb(180, 180, 200)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Updated  ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled(
                updated.format("%Y-%m-%d  %H:%M").to_string(),
                Style::default().fg(Color::Rgb(180, 180, 200)),
            ),
        ]),
    ];

    if let Some(completed_at) = task.completed_at {
        let completed = completed_at.with_timezone(&Local);
        lines.push(Line::from(vec![
            Span::styled("Completed ", Style::default().fg(Color::Rgb(100, 100, 130))),
            Span::styled(
                completed.format("%Y-%m-%d  %H:%M").to_string(),
                Style::default().fg(Color::Rgb(80, 200, 120)),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // Tags
    if !task.tags.is_empty() {
        lines.push(Line::from(Span::styled(
            "Tags",
            Style::default().fg(Color::Rgb(100, 100, 130)),
        )));
        let tag_spans: Vec<Span> = task.tags.iter().flat_map(|t| {
            vec![
                Span::styled(
                    format!(" {} ", t),
                    Style::default()
                        .fg(Color::Rgb(200, 180, 255))
                        .bg(Color::Rgb(40, 30, 70))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
            ]
        }).collect();
        lines.push(Line::from(""));
        lines.push(Line::from(tag_spans));
        lines.push(Line::from(""));
    }

    // Description
    if let Some(desc) = &task.description {
        lines.push(Line::from(Span::styled(
            "Description",
            Style::default().fg(Color::Rgb(100, 100, 130)),
        )));
        lines.push(Line::from(""));
        for dl in desc.lines() {
            lines.push(Line::from(Span::styled(
                dl.to_string(),
                Style::default().fg(Color::Rgb(200, 200, 215)),
            )));
        }
    }

    // ID (dimmed, at bottom)
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("ID  {}", &task.id[..8]),
        Style::default().fg(Color::Rgb(50, 50, 70)),
    )));

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), detail_area);
}

fn draw_input_modal(f: &mut Frame, ov: &PlannerOverlay, parent: Rect, title: &str) {
    // 13 rows: border×2 + pad×2 + 3×(label+field) + hint
    let modal = centered_rect_abs(70, 13, parent);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(160, 130, 255)))
        .style(Style::default().bg(Color::Rgb(18, 15, 30)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Rgb(200, 180, 255))
                .add_modifier(Modifier::BOLD),
        ));

    f.render_widget(block, modal);

    // Read all three stored values from the thread-local scratch
    let (scratch_title, scratch_tags, scratch_desc) = planner_scratch_peek();

    // For the active field, input_buf holds the live-edited value.
    // For inactive fields, we read from scratch.
    let (title_val, tags_val, desc_val) = match ov.input_field {
        InputField::Title       => (ov.input_buf.as_str(), scratch_tags.as_str(),  scratch_desc.as_str()),
        InputField::Tags        => (scratch_title.as_str(), ov.input_buf.as_str(), scratch_desc.as_str()),
        InputField::Description => (scratch_title.as_str(), scratch_tags.as_str(),  ov.input_buf.as_str()),
    };

    // Build a single row: label + text with cursor if active, plain text if not
    let make_row = |buf: &str, cursor: usize, label: &str, active: bool| -> Line {
        let label_style = if active {
            Style::default().fg(Color::Rgb(200, 180, 255)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(70, 60, 100))
        };
        let text_style = if active {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Rgb(100, 90, 130))
        };

        if active {
            let before = &buf[..cursor];
            let cursor_ch = buf.chars().nth(cursor)
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            let after_start = cursor + cursor_ch.len().min(buf.len().saturating_sub(cursor));
            let after = if after_start <= buf.len() { &buf[after_start..] } else { "" };

            Line::from(vec![
                Span::styled(format!("  {}", label), label_style),
                Span::styled(before.to_string(), text_style),
                Span::styled(cursor_ch, Style::default().fg(Color::Black).bg(Color::White)),
                Span::styled(after.to_string(), text_style),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!("  {}", label), label_style),
                Span::styled(buf.to_string(), text_style),
            ])
        }
    };

    let cursor = ov.input_cursor;
    let fields = Paragraph::new(vec![
        Line::from(""),
        make_row(title_val, if ov.input_field == InputField::Title { cursor } else { title_val.len() },
                 "Title*  : ", ov.input_field == InputField::Title),
        Line::from(""),
        make_row(tags_val,  if ov.input_field == InputField::Tags  { cursor } else { tags_val.len() },
                 "Tags    : ", ov.input_field == InputField::Tags),
        Line::from(""),
        make_row(desc_val,  if ov.input_field == InputField::Description { cursor } else { desc_val.len() },
                 "Desc    : ", ov.input_field == InputField::Description),
    ]);

    let fields_area = Rect {
        x: modal.x + 1,
        y: modal.y + 1,
        width: modal.width.saturating_sub(2),
        height: modal.height.saturating_sub(3),
    };
    f.render_widget(fields, fields_area);

    let hint_area = Rect {
        x: modal.x + 2,
        y: modal.y + modal.height.saturating_sub(2),
        width: modal.width.saturating_sub(4),
        height: 1,
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Enter/Tab ", Style::default().fg(Color::Rgb(160, 130, 255))),
            Span::styled("next field  ", Style::default().fg(Color::Rgb(80, 80, 100))),
            Span::styled("Enter ", Style::default().fg(Color::Rgb(160, 130, 255))),
            Span::styled("(on Desc) confirm  ", Style::default().fg(Color::Rgb(80, 80, 100))),
            Span::styled("Esc ", Style::default().fg(Color::Rgb(160, 130, 255))),
            Span::styled("cancel", Style::default().fg(Color::Rgb(80, 80, 100))),
        ])),
        hint_area,
    );
}

fn draw_confirm_modal(f: &mut Frame, ov: &PlannerOverlay, parent: Rect) {
    let task_title = ov.selected_task().map(|t| t.title.as_str()).unwrap_or("this task");
    let modal = centered_rect_abs(50, 7, parent);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(220, 80, 80)))
        .style(Style::default().bg(Color::Rgb(20, 10, 12)))
        .title(Span::styled(
            "  ⚠ Delete Task  ",
            Style::default()
                .fg(Color::Rgb(220, 80, 80))
                .add_modifier(Modifier::BOLD),
        ));

    f.render_widget(block, modal);

    let inner = Rect {
        x: modal.x + 2,
        y: modal.y + 2,
        width: modal.width.saturating_sub(4),
        height: modal.height.saturating_sub(3),
    };

    let max_len = inner.width as usize;
    let trimmed = if task_title.len() > max_len.saturating_sub(16) {
        format!("{}…", &task_title[..max_len.saturating_sub(17)])
    } else {
        task_title.to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Delete  ", Style::default().fg(Color::Rgb(180, 180, 200))),
            Span::styled(trimmed, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(" ?", Style::default().fg(Color::Rgb(180, 180, 200))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("y / Enter ", Style::default().fg(Color::Rgb(220, 80, 80)).add_modifier(Modifier::BOLD)),
            Span::styled("confirm  ", Style::default().fg(Color::Rgb(100, 80, 80))),
            Span::styled("any other key ", Style::default().fg(Color::Rgb(100, 100, 120))),
            Span::styled("cancel", Style::default().fg(Color::Rgb(100, 100, 120))),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
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

fn centered_rect_abs(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect { x, y, width: width.min(r.width), height: height.min(r.height) }
}
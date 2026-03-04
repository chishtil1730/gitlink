/// Generic scrollable info overlay used by: activity, commits, pull-requests,
/// repo-sync, multi-repo, push-check, push-verify, branches, issues, user-info,
/// auth, and prp.
///
/// The caller fills `InfoOverlay::lines` with styled `ratatui::text::Line` values
/// and sets a title string.  The overlay handles scrolling and the Esc/q/↑↓
/// key bindings (handled in app.rs).

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, ov: &crate::tui::app::InfoOverlay) {
    let area = f.area();
    let popup = centered_rect(92, 80, area);

    f.render_widget(Clear, popup);

    let accent = ov.accent;

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 70, 90)))
        .style(Style::default().bg(Color::Rgb(13, 15, 20)))
        .title(Span::styled(
            format!("  {}  ", ov.title),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(
            "  ↑↓ / PgUp PgDn  scroll    q / Esc  close  ",
            Style::default().fg(Color::Rgb(80, 90, 110)),
        ));

    f.render_widget(outer, popup);

    let inner = Rect {
        x: popup.x + 2,
        y: popup.y + 1,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(2),
    };

    let total = ov.lines.len() as u16;
    let visible = inner.height;
    let max_scroll = total.saturating_sub(visible);
    let scroll = (ov.scroll as u16).min(max_scroll);

    f.render_widget(
        Paragraph::new(ov.lines.clone())
            .scroll((scroll, 0))
            .wrap(Wrap { trim: false }),
        inner,
    );

    // Scroll indicator in bottom-right corner of the border
    if total > visible {
        let pct = if max_scroll == 0 { 100 } else { (100 - (scroll * 100 / max_scroll)) as u16 };
        let indicator = Paragraph::new(Line::from(Span::styled(
            format!(" {}% ↕ ", pct),
            Style::default().fg(Color::Rgb(80, 90, 110)),
        )))
            .alignment(Alignment::Right);

        let ind_area = Rect {
            x: popup.x,
            y: popup.y + popup.height.saturating_sub(1),
            width: popup.width.saturating_sub(2),
            height: 1,
        };
        f.render_widget(indicator, ind_area);
    }
}

// ── PRP overlay (two-panel: repo list + commit input) ───────────────────────

pub fn draw_prp(f: &mut Frame, ov: &crate::tui::app::PrpOverlay) {
    use crate::tui::app::PrpStep;

    let area = f.area();
    let popup = centered_rect(92, 86, area);
    f.render_widget(Clear, popup);

    let accent = Color::Rgb(130, 90, 200);

    let hint = match ov.step {
        PrpStep::SelectRepos  => "  ↑↓  navigate    Space  toggle    Enter  review changes    Esc  close  ",
        PrpStep::ReviewChanges=> "  ↑↓ / PgUp PgDn  scroll    Enter  write message    Esc  back  ",
        PrpStep::EnterMessage => "  Type commit message    Enter  confirm    Esc  back  ",
        PrpStep::ConfirmPush  => "  y  push to remote    n / Enter  commit only    Esc  back  ",
        PrpStep::Result       => "  ↑↓ / PgUp PgDn  scroll    Enter / Esc  close  ",
    };

    let title = match ov.step {
        PrpStep::SelectRepos   => "  🔗 PRP Hub — Select Repositories  ",
        PrpStep::ReviewChanges => "  🔗 PRP Hub — Review Changes  ",
        PrpStep::EnterMessage  => "  🔗 PRP Hub — Commit Message  ",
        PrpStep::ConfirmPush   => "  🔗 PRP Hub — Push to Remote?  ",
        PrpStep::Result        => "  🔗 PRP Hub — Result  ",
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 70, 90)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled(title, Style::default().fg(accent).add_modifier(Modifier::BOLD)))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(hint, Style::default().fg(Color::Rgb(70, 80, 100))));

    f.render_widget(outer, popup);

    let inner = Rect {
        x: popup.x + 1,
        y: popup.y + 1,
        width: popup.width.saturating_sub(2),
        height: popup.height.saturating_sub(2),
    };

    match ov.step {
        PrpStep::SelectRepos => {
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(inner);
            draw_prp_repo_list(f, ov, panels[0], accent);
            draw_prp_select_detail(f, ov, panels[1], accent);
        }
        PrpStep::ReviewChanges => {
            draw_prp_diff(f, ov, inner, accent);
        }
        PrpStep::EnterMessage => {
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(inner);
            draw_prp_repo_list(f, ov, panels[0], accent);
            draw_prp_message_input(f, ov, panels[1], accent);
        }
        PrpStep::ConfirmPush => {
            draw_prp_confirm_push(f, ov, inner, accent);
        }
        PrpStep::Result => {
            draw_prp_result(f, ov, inner, accent);
        }
    }
}

fn draw_prp_repo_list(f: &mut Frame, ov: &crate::tui::app::PrpOverlay, area: Rect, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(45, 50, 68)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled("  Repositories  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));

    f.render_widget(block, area);

    let list_area = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    if ov.repos.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No git repositories found.",
                    Style::default().fg(Color::Rgb(90, 95, 115)),
                )),
            ]).alignment(Alignment::Center),
            list_area,
        );
        return;
    }

    let lines: Vec<Line> = ov.repos.iter().enumerate().map(|(i, repo)| {
        let is_cursor   = i == ov.selected;
        let is_included = ov.included[i];

        // Show just the final path component as the display name
        let display = std::path::Path::new(repo)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| repo.clone());

        let pointer = if is_cursor {
            Span::styled("▶ ", Style::default().fg(accent))
        } else {
            Span::raw("  ")
        };

        let checkbox = if is_included {
            Span::styled("☑ ", Style::default().fg(Color::Rgb(80, 210, 130)).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("☐ ", Style::default().fg(Color::Rgb(55, 60, 78)))
        };

        let name_style = if is_cursor {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else if is_included {
            Style::default().fg(Color::Rgb(200, 208, 230))
        } else {
            Style::default().fg(Color::Rgb(75, 80, 100))
        };

        let bg = if is_cursor { Style::default().bg(Color::Rgb(22, 20, 38)) } else { Style::default() };

        Line::from(vec![pointer, checkbox, Span::styled(display, name_style)]).style(bg)
    }).collect();

    f.render_widget(Paragraph::new(lines), list_area);
}

fn draw_prp_select_detail(f: &mut Frame, ov: &crate::tui::app::PrpOverlay, area: Rect, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(45, 50, 68)))
        .style(Style::default().bg(Color::Rgb(12, 14, 20)))
        .title(Span::styled("  Session  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));

    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 3,
        y: area.y + 2,
        width: area.width.saturating_sub(6),
        height: area.height.saturating_sub(3),
    };

    let included = ov.included.iter().filter(|&&b| b).count();
    let total    = ov.repos.len();

    let mut lines = vec![
        Line::from(Span::styled(
            "Select repositories for this commit session.",
            Style::default().fg(Color::Rgb(150, 158, 185)),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Selected  ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(
                format!("{} / {}", included, total),
                if included > 0 {
                    Style::default().fg(Color::Rgb(80, 210, 130)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(90, 100, 130))
                },
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ──────────────────────────────────",
            Style::default().fg(Color::Rgb(38, 42, 58)),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space  ", Style::default().fg(accent)),
            Span::styled("toggle repo", Style::default().fg(Color::Rgb(140, 148, 172))),
        ]),
        Line::from(vec![
            Span::styled("  Enter  ", Style::default().fg(accent)),
            Span::styled("review changes", Style::default().fg(Color::Rgb(140, 148, 172))),
        ]),
        Line::from(vec![
            Span::styled("  Esc    ", Style::default().fg(accent)),
            Span::styled("cancel session", Style::default().fg(Color::Rgb(140, 148, 172))),
        ]),
    ];

    if included > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ──────────────────────────────────",
            Style::default().fg(Color::Rgb(38, 42, 58)),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {} repo{} ready", included, if included == 1 { "" } else { "s" }),
            Style::default().fg(Color::Rgb(80, 210, 130)).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  Press Enter to see changes →",
            Style::default().fg(Color::Rgb(80, 210, 130)),
        )));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_prp_diff(f: &mut Frame, ov: &crate::tui::app::PrpOverlay, area: Rect, accent: Color) {
    use crate::tui::app::DiffKind;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(45, 50, 68)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled("  Changes  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));

    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let visible = inner.height as usize;
    let total   = ov.diff_lines.len();
    let max_scroll = total.saturating_sub(visible);
    let scroll = ov.diff_scroll.min(max_scroll);

    let lines: Vec<Line> = ov.diff_lines.iter()
        .skip(scroll)
        .take(visible)
        .map(|(text, kind)| {
            let style = match kind {
                DiffKind::Header   => Style::default().fg(Color::Rgb(160, 120, 240)).add_modifier(Modifier::BOLD),
                DiffKind::Added    => Style::default().fg(Color::Rgb(80, 210, 130)),
                DiffKind::Removed  => Style::default().fg(Color::Rgb(220, 80, 80)),
                DiffKind::Modified => Style::default().fg(Color::Rgb(230, 180, 60)),
                DiffKind::Stat     => Style::default().fg(Color::Rgb(100, 155, 245)),
                DiffKind::Neutral  => Style::default().fg(Color::Rgb(70, 78, 100)),
            };
            Line::from(Span::styled(text.clone(), style))
        })
        .collect();

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

    // Scroll position indicator
    if total > visible {
        let pct = if total <= 1 { 100u16 } else { ((scroll * 100) / (total - 1)) as u16 };
        let hint = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  ↑↓ scroll  {}/{} ", scroll + 1, total),
                Style::default().fg(Color::Rgb(60, 68, 88)),
            ),
            Span::styled(
                format!("{}%  Enter to commit →  ", pct),
                Style::default().fg(Color::Rgb(100, 108, 138)),
            ),
        ])).alignment(Alignment::Right);

        let hint_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        f.render_widget(hint, hint_area);
    }
}

fn draw_prp_message_input(f: &mut Frame, ov: &crate::tui::app::PrpOverlay, area: Rect, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(Color::Rgb(12, 14, 20)))
        .title(Span::styled("  Commit Message  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));

    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 3,
        y: area.y + 2,
        width: area.width.saturating_sub(6),
        height: area.height.saturating_sub(3),
    };

    let cursor = ov.input_cursor;
    let buf    = &ov.input_buf;
    let before = &buf[..cursor];
    let cursor_ch = buf.chars().nth(cursor)
        .map(|c| c.to_string())
        .unwrap_or_else(|| " ".to_string());
    let after_start = (cursor + cursor_ch.len()).min(buf.len());
    let after  = &buf[after_start..];

    let char_count = buf.chars().count();
    let count_color = if char_count > 72 { Color::Rgb(220, 80, 80) }
    else if char_count > 50 { Color::Rgb(230, 180, 60) }
    else { Color::Rgb(70, 78, 100) };

    let lines = vec![
        Line::from(Span::styled(
            "Describe what you changed:",
            Style::default().fg(Color::Rgb(140, 148, 175)),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  ❯ "),
            Span::styled(before.to_string(), Style::default().fg(Color::White)),
            Span::styled(
                cursor_ch,
                Style::default().fg(Color::Rgb(10, 12, 18)).bg(Color::Rgb(180, 140, 255)),
            ),
            Span::styled(after.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─────────────────────────────────────────",
            Style::default().fg(Color::Rgb(38, 42, 58)),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} chars", char_count), Style::default().fg(count_color)),
            Span::styled("    (72 recommended max)", Style::default().fg(Color::Rgb(55, 60, 78))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter  ", Style::default().fg(accent)),
            Span::styled("next step", Style::default().fg(Color::Rgb(140, 148, 172))),
        ]),
        Line::from(vec![
            Span::styled("  Esc    ", Style::default().fg(accent)),
            Span::styled("back to diff", Style::default().fg(Color::Rgb(140, 148, 172))),
        ]),
    ];

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_prp_confirm_push(f: &mut Frame, ov: &crate::tui::app::PrpOverlay, area: Rect, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)));

    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 4,
        y: area.y + 2,
        width: area.width.saturating_sub(8),
        height: area.height.saturating_sub(4),
    };

    let included: Vec<&String> = ov.repos.iter().enumerate()
        .filter(|(i, _)| ov.included[*i])
        .map(|(_, r)| r)
        .collect();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Ready to commit",
            Style::default().fg(Color::Rgb(180, 190, 255)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Message:  ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(
                ov.input_buf.trim().to_string(),
                Style::default().fg(Color::Rgb(220, 225, 255)).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Repos:    ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(
                format!("{} selected", included.len()),
                Style::default().fg(Color::Rgb(80, 210, 130)),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ─────────────────────────────────────────────────────",
            Style::default().fg(Color::Rgb(38, 42, 58)),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Push to remote after committing?",
            Style::default().fg(Color::Rgb(200, 208, 230)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [ y ]  ", Style::default()
                .fg(Color::Rgb(80, 210, 130))
                .add_modifier(Modifier::BOLD)),
            Span::styled("Yes — commit + push", Style::default().fg(Color::Rgb(80, 210, 130))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [ n ]  ", Style::default()
                .fg(Color::Rgb(160, 120, 240))
                .add_modifier(Modifier::BOLD)),
            Span::styled("No  — commit only", Style::default().fg(Color::Rgb(140, 148, 175))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [ Esc ]  ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled("Back to message", Style::default().fg(Color::Rgb(80, 88, 112))),
        ]),
    ];

    // Show which repos will be committed
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────────────────",
        Style::default().fg(Color::Rgb(38, 42, 58)),
    )));
    lines.push(Line::from(""));
    for repo in &included {
        let name = std::path::Path::new(repo)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| repo.to_string());
        lines.push(Line::from(vec![
            Span::styled("  ▣  ", Style::default().fg(Color::Rgb(100, 155, 245))),
            Span::styled(name, Style::default().fg(Color::Rgb(180, 188, 215))),
        ]));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_prp_result(f: &mut Frame, ov: &crate::tui::app::PrpOverlay, area: Rect, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(45, 50, 68)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled("  Result  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));

    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let visible   = inner.height as usize;
    let total     = ov.result_lines.len();
    let max_scroll = total.saturating_sub(visible);
    let scroll    = ov.diff_scroll.min(max_scroll);

    let lines: Vec<Line> = ov.result_lines.iter()
        .skip(scroll)
        .take(visible)
        .map(|text| {
            let style = if text.contains("✔") {
                Style::default().fg(Color::Rgb(80, 210, 130))
            } else if text.contains("✖") || text.contains("failed") || text.contains("error") {
                Style::default().fg(Color::Rgb(220, 80, 80))
            } else if text.contains("⚠") {
                Style::default().fg(Color::Rgb(230, 180, 60))
            } else if text.contains("▣") {
                Style::default().fg(Color::Rgb(160, 120, 240)).add_modifier(Modifier::BOLD)
            } else if text.contains("Message:") || text.contains("Push") {
                Style::default().fg(Color::Rgb(100, 155, 245))
            } else if text.starts_with("─") || text.starts_with("  ─") {
                Style::default().fg(Color::Rgb(40, 45, 60))
            } else {
                Style::default().fg(Color::Rgb(170, 178, 210))
            };
            Line::from(Span::styled(text.clone(), style))
        })
        .collect();

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}


// ── Auth overlay ─────────────────────────────────────────────────────────────

pub fn draw_auth(f: &mut Frame, ov: &crate::tui::app::AuthOverlay) {
    use crate::tui::app::AuthStep;

    let area = f.area();
    let accent = Color::Rgb(80, 180, 120);

    // ShowCode gets a larger popup so the code is prominent
    let popup = match &ov.step {
        AuthStep::ShowCode { .. } | AuthStep::Polling { .. } => centered_rect(78, 70, area),
        _ => centered_rect(72, 60, area),
    };

    f.render_widget(Clear, popup);

    let hint = match &ov.step {
        AuthStep::Menu => "  ↑↓  select    Enter  confirm    Esc  close  ",
        AuthStep::ShowCode { .. } => "  Opening browser in 5s…    Esc  cancel  ",
        AuthStep::Polling { .. } => "  Waiting for authorization    Esc  cancel  ",
        AuthStep::Result(_) => "  Enter / Esc  close  ",
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 70, 90)))
        .style(Style::default().bg(Color::Rgb(13, 15, 20)))
        .title(Span::styled(
            "  🔐 GitHub Authentication  ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(
            hint.to_string(),
            Style::default().fg(Color::Rgb(80, 90, 110)),
        ));

    f.render_widget(outer, popup);

    let inner = Rect {
        x: popup.x + 2,
        y: popup.y + 2,
        width: popup.width.saturating_sub(4),
        height: popup.height.saturating_sub(3),
    };

    match &ov.step {
        AuthStep::Menu => {
            let options = ["Login via GitHub OAuth", "Logout (remove token)", "Check auth status"];
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    "Choose an action:",
                    Style::default().fg(Color::Rgb(160, 165, 185)),
                )),
                Line::from(""),
            ];
            for (i, opt) in options.iter().enumerate() {
                let is_sel = i == ov.selected;
                let pointer = if is_sel {
                    Span::styled("  ▶  ", Style::default().fg(accent))
                } else {
                    Span::raw("     ")
                };
                let style = if is_sel {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD).bg(Color::Rgb(20, 35, 28))
                } else {
                    Style::default().fg(Color::Rgb(160, 165, 185))
                };
                lines.push(Line::from(vec![pointer, Span::styled(opt.to_string(), style)]));
                lines.push(Line::from(""));
            }
            f.render_widget(Paragraph::new(lines), inner);
        }

        AuthStep::ShowCode { user_code, url } => {
            draw_auth_code_screen(f, inner, accent, user_code, url, None);
        }

        AuthStep::Polling { user_code, url, frame } => {
            let frames = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
            let spinner = frames[frame % frames.len()];
            draw_auth_code_screen(f, inner, accent, user_code, url, Some(spinner));
        }

        AuthStep::Result(msg) => {
            let is_ok = !msg.starts_with("Error") && !msg.starts_with("Not");
            let color = if is_ok { Color::Rgb(46, 160, 90) } else { Color::Rgb(200, 80, 80) };
            let icon = if is_ok { "✔" } else { "✖" };
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}  {}", icon, msg),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter or Esc to close.",
                    Style::default().fg(Color::Rgb(80, 90, 110)),
                )),
            ];
            f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

fn big_char(c: char) -> [&'static str; 5] {
    match c {
        '0' => ["█████", "█   █", "█   █", "█   █", "█████"],
        '1' => ["  ██ ", "   █ ", "   █ ", "   █ ", "█████"],
        '2' => ["█████", "    █", "█████", "█    ", "█████"],
        '3' => ["█████", "    █", " ████", "    █", "█████"],
        '4' => ["█   █", "█   █", "█████", "    █", "    █"],
        '5' => ["█████", "█    ", "█████", "    █", "█████"],
        '6' => ["█████", "█    ", "█████", "█   █", "█████"],
        '7' => ["█████", "    █", "   █ ", "  █  ", "  █  "],
        '8' => ["█████", "█   █", "█████", "█   █", "█████"],
        '9' => ["█████", "█   █", "█████", "    █", "█████"],
        'A' => [" ███ ", "█   █", "█████", "█   █", "█   █"],
        'B' => ["████ ", "█   █", "████ ", "█   █", "████ "],
        'C' => ["█████", "█    ", "█    ", "█    ", "█████"],
        'D' => ["████ ", "█   █", "█   █", "█   █", "████ "],
        'E' => ["█████", "█    ", "████ ", "█    ", "█████"],
        'F' => ["█████", "█    ", "████ ", "█    ", "█    "],
        'G' => ["█████", "█    ", "█  ██", "█   █", "█████"],
        'H' => ["█   █", "█   █", "█████", "█   █", "█   █"],
        'I' => ["█████", "  █  ", "  █  ", "  █  ", "█████"],
        'J' => ["█████", "    █", "    █", "█   █", " ████"],
        'K' => ["█   █", "█  █ ", "███  ", "█  █ ", "█   █"],
        'L' => ["█    ", "█    ", "█    ", "█    ", "█████"],
        'M' => ["█   █", "██ ██", "█ █ █", "█   █", "█   █"],
        'N' => ["█   █", "██  █", "█ █ █", "█  ██", "█   █"],
        'O' => [" ███ ", "█   █", "█   █", "█   █", " ███ "],
        'P' => ["████ ", "█   █", "████ ", "█    ", "█    "],
        'Q' => [" ███ ", "█   █", "█ █ █", "█  ██", " ████"],
        'R' => ["████ ", "█   █", "████ ", "█  █ ", "█   █"],
        'S' => ["█████", "█    ", "█████", "    █", "█████"],
        'T' => ["█████", "  █  ", "  █  ", "  █  ", "  █  "],
        'U' => ["█   █", "█   █", "█   █", "█   █", "█████"],
        'V' => ["█   █", "█   █", "█   █", " █ █ ", "  █  "],
        'W' => ["█   █", "█   █", "█ █ █", "██ ██", "█   █"],
        'X' => ["█   █", " █ █ ", "  █  ", " █ █ ", "█   █"],
        'Y' => ["█   █", " █ █ ", "  █  ", "  █  ", "  █  "],
        'Z' => ["█████", "   █ ", "  █  ", " █   ", "█████"],
        '-' => ["     ", "     ", "█████", "     ", "     "],
        ' ' => ["     ", "     ", "     ", "     ", "     "],
        _   => ["     ", "  █  ", "     ", "  █  ", "     "],
    }
}

fn render_big_text(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.to_uppercase().chars().collect();
    let mut rows = vec![String::new(); 5];
    for (i, ch) in chars.iter().enumerate() {
        let glyph = big_char(*ch);
        for row in 0..5 {
            if i > 0 { rows[row].push(' '); }
            rows[row].push_str(glyph[row]);
        }
    }
    rows
}

fn draw_auth_code_screen(
    f: &mut Frame,
    inner: Rect,
    accent: Color,
    user_code: &str,
    url: &str,
    spinner: Option<&str>,
) {
    // Big text is 5 rows tall + 2 border = 7 for the code box
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1), // label
            Constraint::Length(7), // code box (5 rows + top/bottom border)
            Constraint::Length(1),
            Constraint::Length(1), // url
            Constraint::Length(1),
            Constraint::Min(0),    // status
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(
            "Enter this code at the URL below:",
            Style::default().fg(Color::Rgb(160, 165, 185)),
        )),
        chunks[1],
    );

    let code_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Color::Rgb(18, 28, 22)));
    f.render_widget(code_block, chunks[2]);

    let code_inner = Rect {
        x: chunks[2].x + 1,
        y: chunks[2].y + 1,
        width: chunks[2].width.saturating_sub(2),
        height: 5,
    };

    let big_rows = render_big_text(user_code);
    let lines: Vec<Line> = big_rows
        .iter()
        .map(|row| {
            Line::from(Span::styled(
                row.clone(),
                Style::default()
                    .fg(Color::Rgb(80, 255, 140))
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        code_inner,
    );

    if !url.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  URL: ", Style::default().fg(Color::Rgb(110, 115, 130))),
                Span::styled(
                    url.to_string(),
                    Style::default().fg(Color::Rgb(100, 149, 237)).add_modifier(Modifier::UNDERLINED),
                ),
            ])),
            chunks[4],
        );
    }

    let status_line = match spinner {
        None => Line::from(Span::styled(
            "  Browser will open in 5 seconds. Authorize in the browser to continue.",
            Style::default().fg(Color::Rgb(160, 165, 185)),
        )),
        Some(s) => Line::from(vec![
            Span::styled(
                format!("  {} ", s),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Waiting for authorization…",
                Style::default().fg(Color::Rgb(160, 165, 185)),
            ),
        ]),
    };
    f.render_widget(Paragraph::new(status_line), chunks[6]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let v = Layout::default()
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
        .split(v[1])[1]
}
// ── MultiSync overlay ────────────────────────────────────────────────────────

pub fn draw_multi_sync(f: &mut Frame, ov: &crate::tui::app::MultiSyncOverlay) {
    use crate::tui::app::MultiSyncStep;

    let area = f.area();
    let popup = centered_rect(94, 88, area);
    f.render_widget(Clear, popup);

    let accent = Color::Rgb(100, 180, 200);

    let hint = match ov.step {
        MultiSyncStep::Loading  => "  Loading repositories…    Esc  cancel  ",
        MultiSyncStep::Running  => "  Checking sync status…    Esc  cancel  ",
        MultiSyncStep::SelectRepos => "  ↑↓  navigate    Space  toggle    a  all/none    /  search    Enter  run sync    Esc  close  ",
        MultiSyncStep::Results  => "  ↑↓ / PgUp PgDn  scroll    q / Esc  close  ",
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 70, 90)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled(
            "  📦 Multi-Repo Sync  ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Left)
        .title_bottom(Span::styled(
            hint,
            Style::default().fg(Color::Rgb(70, 80, 100)),
        ));

    f.render_widget(outer, popup);

    let inner = Rect {
        x: popup.x + 1,
        y: popup.y + 1,
        width: popup.width.saturating_sub(2),
        height: popup.height.saturating_sub(2),
    };

    match ov.step {
        MultiSyncStep::Loading | MultiSyncStep::Running => {
            draw_multi_sync_loading(f, ov, inner, accent);
        }
        MultiSyncStep::SelectRepos => {
            draw_multi_sync_select(f, ov, inner, accent);
        }
        MultiSyncStep::Results => {
            draw_multi_sync_results(f, ov, inner);
        }
    }
}

fn draw_multi_sync_loading(f: &mut Frame, ov: &crate::tui::app::MultiSyncOverlay, area: Rect, accent: Color) {
    use crate::tui::app::MultiSyncStep;
    let msg = match ov.step {
        MultiSyncStep::Loading => "  Fetching your GitHub repositories…",
        _ =>                      "  Checking sync status for selected repositories…",
    };
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(msg, Style::default().fg(Color::Rgb(160, 170, 200)))),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_multi_sync_select(f: &mut Frame, ov: &crate::tui::app::MultiSyncOverlay, area: Rect, accent: Color) {
    // Split: left = repo list, right = legend/stats
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(area);

    draw_multi_sync_list(f, ov, panels[0], accent);
    draw_multi_sync_sidebar(f, ov, panels[1], accent);
}

fn draw_multi_sync_list(f: &mut Frame, ov: &crate::tui::app::MultiSyncOverlay, area: Rect, accent: Color) {
    let border_color = Color::Rgb(45, 52, 68);

    // Search bar height
    let search_h = if ov.search_active || !ov.search.is_empty() { 3u16 } else { 0u16 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(search_h),
            Constraint::Min(0),
        ])
        .split(area);

    // Search bar
    if search_h > 0 {
        let query_display = format!("  / {}▌", ov.search);
        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent))
            .style(Style::default().bg(Color::Rgb(10, 14, 22)));
        f.render_widget(search_block, chunks[0]);
        let inner_search = Rect {
            x: chunks[0].x + 2,
            y: chunks[0].y + 1,
            width: chunks[0].width.saturating_sub(4),
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Span::styled(query_display, Style::default().fg(accent))),
            inner_search,
        );
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled("  Repositories  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));
    f.render_widget(&list_block, chunks[1]);

    let list_inner = Rect {
        x: chunks[1].x + 2,
        y: chunks[1].y + 1,
        width: chunks[1].width.saturating_sub(4),
        height: chunks[1].height.saturating_sub(2),
    };

    let filtered = ov.filtered_indices();

    if filtered.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No repositories match your search.",
                    Style::default().fg(Color::Rgb(80, 85, 110)),
                )),
            ]),
            list_inner,
        );
        return;
    }

    // Scroll to keep cursor visible
    let visible_h = list_inner.height as usize;
    let scroll_start = if ov.cursor >= visible_h {
        ov.cursor - visible_h + 1
    } else {
        0
    };

    let lines: Vec<Line> = filtered.iter().enumerate()
        .skip(scroll_start)
        .take(visible_h)
        .map(|(vis_idx, &real_idx)| {
            let repo = &ov.repos[real_idx];
            let cursor_idx = vis_idx; // position in filtered list
            let is_cursor = ov.cursor == (scroll_start + cursor_idx).min(filtered.len().saturating_sub(1));
            // re-derive correctly:
            let actual_filtered_pos = scroll_start + cursor_idx;
            let is_cursor = ov.cursor == actual_filtered_pos;

            let checkbox = if repo.selected {
                Span::styled("☑ ", Style::default().fg(Color::Rgb(80, 210, 130)).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("☐ ", Style::default().fg(Color::Rgb(60, 65, 88)))
            };

            let privacy_icon = if repo.is_private {
                Span::styled("🔒 ", Style::default())
            } else {
                Span::styled("🌍 ", Style::default())
            };

            let name_style = if is_cursor {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            } else if repo.selected {
                Style::default().fg(Color::Rgb(210, 218, 245))
            } else {
                Style::default().fg(Color::Rgb(130, 138, 165))
            };

            let bg = if is_cursor {
                Style::default().bg(Color::Rgb(25, 30, 50))
            } else {
                Style::default()
            };

            let pointer = if is_cursor {
                Span::styled("▶ ", Style::default().fg(accent))
            } else {
                Span::raw("  ")
            };

            // Truncate description
            let desc = if repo.description.is_empty() {
                String::new()
            } else {
                let max_desc = (list_inner.width as usize).saturating_sub(50);
                if repo.description.len() > max_desc && max_desc > 3 {
                    format!("  {}", &repo.description[..max_desc])
                } else {
                    format!("  {}", repo.description)
                }
            };

            Line::from(vec![
                pointer,
                checkbox,
                privacy_icon,
                Span::styled(repo.name_with_owner.clone(), name_style),
                Span::styled(desc, Style::default().fg(Color::Rgb(70, 78, 100))),
            ]).style(bg)
        })
        .collect();

    f.render_widget(Paragraph::new(lines), list_inner);

    // Scroll indicator
    if filtered.len() > visible_h {
        let pct = if filtered.len() <= 1 { 100u16 }
        else { (ov.cursor * 100 / (filtered.len() - 1)) as u16 };
        let ind = Paragraph::new(Line::from(Span::styled(
            format!(" {}% ", pct),
            Style::default().fg(Color::Rgb(70, 78, 100)),
        ))).alignment(Alignment::Right);
        let ind_area = Rect {
            x: chunks[1].x,
            y: chunks[1].y + chunks[1].height.saturating_sub(1),
            width: chunks[1].width.saturating_sub(2),
            height: 1,
        };
        f.render_widget(ind, ind_area);
    }
}

fn draw_multi_sync_sidebar(f: &mut Frame, ov: &crate::tui::app::MultiSyncOverlay, area: Rect, accent: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(45, 52, 68)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled("  Selection  ", Style::default().fg(accent).add_modifier(Modifier::BOLD)));
    f.render_widget(&block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(3),
    };

    let total     = ov.repos.len();
    let selected  = ov.repos.iter().filter(|r| r.selected).count();
    let filtered  = ov.filtered_indices().len();
    let private   = ov.repos.iter().filter(|r| r.is_private).count();
    let public    = total - private;

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Total      ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(format!("{}", total), Style::default().fg(Color::Rgb(180, 190, 220)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Selected   ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(
                format!("{}/{}", selected, total),
                if selected > 0 {
                    Style::default().fg(Color::Rgb(80, 210, 130)).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(90, 100, 130))
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Visible    ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(format!("{}", filtered), Style::default().fg(Color::Rgb(180, 190, 220))),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("🌍 Public  ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(format!("{}", public), Style::default().fg(Color::Rgb(100, 155, 245))),
        ]),
        Line::from(vec![
            Span::styled("🔒 Private ", Style::default().fg(Color::Rgb(90, 100, 130))),
            Span::styled(format!("{}", private), Style::default().fg(Color::Rgb(160, 120, 240))),
        ]),
        Line::from(""),
        Line::from(Span::styled("─────────────────", Style::default().fg(Color::Rgb(40, 45, 60)))),
        Line::from(""),
        Line::from(Span::styled("Keys:", Style::default().fg(Color::Rgb(90, 100, 130)).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space  ", Style::default().fg(accent)),
            Span::styled("toggle", Style::default().fg(Color::Rgb(140, 148, 175))),
        ]),
        Line::from(vec![
            Span::styled("  a      ", Style::default().fg(accent)),
            Span::styled("all / none", Style::default().fg(Color::Rgb(140, 148, 175))),
        ]),
        Line::from(vec![
            Span::styled("  /      ", Style::default().fg(accent)),
            Span::styled("search", Style::default().fg(Color::Rgb(140, 148, 175))),
        ]),
        Line::from(vec![
            Span::styled("  Enter  ", Style::default().fg(accent)),
            Span::styled("run sync", Style::default().fg(Color::Rgb(140, 148, 175))),
        ]),
        Line::from(vec![
            Span::styled("  Esc    ", Style::default().fg(accent)),
            Span::styled("close", Style::default().fg(Color::Rgb(140, 148, 175))),
        ]),
    ];

    if selected > 0 {
        let mut all_lines = lines;
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled("─────────────────", Style::default().fg(Color::Rgb(40, 45, 60)))));
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled(
            format!("  {} ready to sync", selected),
            Style::default().fg(Color::Rgb(80, 210, 130)).add_modifier(Modifier::BOLD),
        )));
        all_lines.push(Line::from(Span::styled(
            "  Press Enter",
            Style::default().fg(Color::Rgb(80, 210, 130)),
        )));
        f.render_widget(Paragraph::new(all_lines), inner);
    } else {
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn draw_multi_sync_results(f: &mut Frame, ov: &crate::tui::app::MultiSyncOverlay, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(45, 52, 68)))
        .style(Style::default().bg(Color::Rgb(10, 12, 18)))
        .title(Span::styled("  Sync Results  ", Style::default().fg(Color::Rgb(100, 180, 200)).add_modifier(Modifier::BOLD)));
    f.render_widget(&block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let visible = inner.height as usize;
    let max_scroll = ov.result_lines.len().saturating_sub(visible);
    let scroll = ov.scroll.min(max_scroll);

    let lines: Vec<Line> = ov.result_lines.iter()
        .skip(scroll)
        .take(visible)
        .map(|(text, color)| Line::from(Span::styled(text.clone(), Style::default().fg(*color))))
        .collect();

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
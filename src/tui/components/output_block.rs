use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::tui::app::{OutputBlock, OutputKind};

// Box-drawing chars
const TL: &str = "╭";
const TR: &str = "╮";
const BL: &str = "╰";
const BR: &str = "╯";
const H:  &str = "─";
const V:  &str = "│";

pub fn render_lines(blocks: &[OutputBlock]) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Group blocks into execution sessions:
    // A session starts with a Command block, followed by any non-Command blocks.
    // Leading non-Command blocks (e.g. the welcome Info) each get their own mini box.
    let mut i = 0;
    while i < blocks.len() {
        match blocks[i].kind {
            OutputKind::Command => {
                // Collect the command + all following response blocks
                let cmd = &blocks[i];
                let mut responses: Vec<&OutputBlock> = Vec::new();
                let mut j = i + 1;
                while j < blocks.len() && blocks[j].kind != OutputKind::Command {
                    responses.push(&blocks[j]);
                    j += 1;
                }
                render_session(&mut lines, cmd, &responses);
                i = j;
            }
            _ => {
                // Standalone block (e.g. welcome message) — its own box
                render_standalone(&mut lines, &blocks[i]);
                i += 1;
            }
        }
        lines.push(Line::from(""));
    }

    lines
}

/// Renders a command + its responses as a single bordered session box.
fn render_session(lines: &mut Vec<Line<'static>>, cmd: &OutputBlock, responses: &[&OutputBlock]) {
    let border_color = Color::Rgb(55, 50, 80);
    let cmd_prefix_style = Style::default()
        .fg(Color::Rgb(160, 120, 220))
        .add_modifier(Modifier::BOLD);
    let cmd_text_style = Style::default()
        .fg(Color::Rgb(220, 215, 255))
        .add_modifier(Modifier::BOLD);

    // ── Top border with command label ────────────────────────────────────────
    let label = format!(" {} ", cmd.content.trim());
    // Fixed width top border — label embedded
    lines.push(Line::from(vec![
        Span::styled(TL.to_string(), Style::default().fg(border_color)),
        Span::styled(
            format!("{} ", H.repeat(1)),
            Style::default().fg(border_color),
        ),
        Span::styled("> ", cmd_prefix_style),
        Span::styled(cmd.content.trim().to_string(), cmd_text_style),
        Span::styled(format!(" {}", H.repeat(2)), Style::default().fg(border_color)),
        Span::styled(TR.to_string(), Style::default().fg(border_color)),
    ]));

    // ── Response lines inside the box ────────────────────────────────────────
    if responses.is_empty() {
        // Empty response — just a bottom border
        lines.push(Line::from(vec![
            Span::styled(V.to_string(), Style::default().fg(border_color)),
            Span::styled(
                "  no output",
                Style::default().fg(Color::Rgb(60, 60, 80)),
            ),
        ]));
    } else {
        for block in responses {
            let content_lines: Vec<&str> = block.content.lines().collect();
            let (prefix, prefix_style, content_style) = block_styles(block);

            for (idx, content_line) in content_lines.iter().enumerate() {
                let pfx = if idx == 0 { prefix } else { "  " };
                let pfx_style = if idx == 0 { prefix_style } else { Style::default() };
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", V), Style::default().fg(border_color)),
                    Span::styled(pfx.to_string(), pfx_style),
                    Span::styled(content_line.to_string(), content_style),
                ]));
            }
        }
    }

    // ── Bottom border ─────────────────────────────────────────────────────────
    lines.push(Line::from(vec![
        Span::styled(BL.to_string(), Style::default().fg(border_color)),
        Span::styled(H.repeat(30), Style::default().fg(border_color)),
        Span::styled(BR.to_string(), Style::default().fg(border_color)),
    ]));
}

/// Renders a standalone (non-command) block — welcome message etc.
fn render_standalone(lines: &mut Vec<Line<'static>>, block: &OutputBlock) {
    let border_color = Color::Rgb(45, 45, 65);
    let content_lines: Vec<&str> = block.content.lines().collect();
    let (prefix, prefix_style, content_style) = block_styles(block);

    lines.push(Line::from(vec![
        Span::styled(TL.to_string(), Style::default().fg(border_color)),
        Span::styled(H.repeat(2), Style::default().fg(border_color)),
        Span::styled(TR.to_string(), Style::default().fg(border_color)),
    ]));

    for (idx, content_line) in content_lines.iter().enumerate() {
        let pfx = if idx == 0 { prefix } else { "  " };
        let pfx_style = if idx == 0 { prefix_style } else { Style::default() };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", V), Style::default().fg(border_color)),
            Span::styled(pfx.to_string(), pfx_style),
            Span::styled(content_line.to_string(), content_style),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled(BL.to_string(), Style::default().fg(border_color)),
        Span::styled(H.repeat(2), Style::default().fg(border_color)),
        Span::styled(BR.to_string(), Style::default().fg(border_color)),
    ]));
}

fn block_styles(block: &OutputBlock) -> (&'static str, Style, Style) {
    match block.kind {
        OutputKind::Command => (
            "> ",
            Style::default().fg(Color::Rgb(160, 120, 220)).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(200, 200, 210)).add_modifier(Modifier::BOLD),
        ),
        OutputKind::Success => (
            "✔ ",
            Style::default().fg(Color::Rgb(100, 220, 120)).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(160, 230, 170)),
        ),
        OutputKind::Error => (
            "✖ ",
            Style::default().fg(Color::Rgb(220, 80, 80)).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(230, 100, 100)),
        ),
        OutputKind::Info => (
            "◆ ",
            Style::default().fg(Color::Rgb(120, 170, 255)).add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Rgb(150, 190, 240)),
        ),
    }
}
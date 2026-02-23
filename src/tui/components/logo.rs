use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

const LOGO: &[&str] = &[
    "       .    ▒▒▒▒▒▒▒▒▒▒▒   ▒▒▒▒▒  ▒▒▒▒▒▒▒▒▒▒▒ ▒▒▒▒▒     . ▒▒▒▒▒ ▒▒▒▒▒▒   ▒▒▒▒ ▒▒▒▒▒  ▒▒▒▒▒ + ▒▒▒▒▒▒▒▒▒▒. ",
    "  .        ▒▒█████████▒▒  ████▒  ██████████▒ ████▒       ████▒ █████▒   ███▒ ████▒  ████▒  .█████████▒ *",
    "      +    ▒███▒▒▒▒▒▒▒▒   ████▒  ▒▒▒████▒▒▒▒ ████▒ *     ████▒ ██████▒  ███▒ ████▒ ████▒▒          ▒███▒",
    "  .        ▒███▒          ████▒     ████▒    ████▒       ████▒ ████▒██▒ ███▒ █████████▒▒       *   ▒███▒",
    "        *  ▒███▒   █████▒ ████▒     ████▒    ████▒  +    ████▒ ████▒▒██▒███▒ ████▒▒████▒  .     ▒██████▒",
    "   .       ▒███▒   ▒▒███▒ ████▒     ████▒    ████▒       ████▒ ████▒ ▒█████▒ ████▒ ▒████▒          ▒███▒",
    "        .  ▒███▒    ███▒▒ ████▒     ████▒    ████▒       ████▒ ████▒  ▒████▒ ████▒  ▒████▒  ▒███▒  ▒███▒",
    "    +      ▒██████████▒▒  ████▒     ████▒    ██████████▒ ████▒ ████▒   ▒███▒ ████▒   ▒████▒ ▒██████████▒ ",
    "  .        ▒▒▒▒▒▒▒▒▒▒▒▒▒  ▒▒▒▒▒     ▒▒▒▒▒    ▒▒▒▒▒▒▒▒▒▒▒ ▒▒▒▒▒ ▒▒▒▒▒   ▒▒▒▒▒ ▒▒▒▒▒   ▒▒▒▒▒▒ ▒▒▒▒▒▒▒▒▒▒▒▒",
    "        .           * .                      .           +         .         ",
];

pub fn logo_height() -> u16 {
    LOGO.len() as u16
}

pub fn render(elapsed: f32) -> Paragraph<'static> {
    let width = LOGO[0].chars().count();
    let height = LOGO.len();
    let drift = elapsed * 0.05;

    let lines: Vec<Line> = LOGO
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let spans: Vec<Span> = row
                .chars()
                .enumerate()
                .map(|(col_index, ch)| {
                    let style = match ch {
                        ' ' => Style::default(),
                        '.' | '*' | '+' => {
                            let pulse =
                                ((elapsed * 2.8 + (col_index as f32 * 0.5)).sin() + 1.0) / 2.0;
                            let v = (80.0 + 150.0 * pulse).min(255.0) as u8;
                            Style::default().fg(Color::Rgb(v, v, v))
                        }
                        _ => {
                            // Shimmer sweep
                            let shimmer_width = 25.0;
                            let shimmer_pos = (elapsed * 0.30 % 1.5) * width as f32;
                            let dist = (col_index as f32 - shimmer_pos).abs();
                            let shimmer_factor = if dist < shimmer_width {
                                1.0 + 0.3 * (1.0 - dist / shimmer_width).powi(2)
                            } else {
                                1.0
                            };

                            let (r, g, b) = smooth_gradient(col_index, width, drift);
                            let (r, g, b) = apply_vertical_depth((r, g, b), row_index, height);

                            let r = (r as f32 * shimmer_factor).min(255.0) as u8;
                            let g = (g as f32 * shimmer_factor).min(255.0) as u8;
                            let b = (b as f32 * shimmer_factor).min(255.0) as u8;

                            Style::default().fg(Color::Rgb(r, g, b))
                        }
                    };
                    Span::styled(ch.to_string(), style)
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    Paragraph::new(lines).alignment(Alignment::Center)
}

fn smooth_gradient(x: usize, width: usize, drift: f32) -> (u8, u8, u8) {
    let ratio = ((x as f32 / width as f32) + drift).fract();
    let colors = [
        (70u8, 190u8, 255u8),
        (120u8, 235u8, 190u8),
        (220u8, 230u8, 120u8),
        (70u8, 190u8, 255u8),
    ];
    let n = (colors.len() - 1) as f32;
    let scaled = ratio * n;
    let idx = scaled.floor() as usize;
    let next = (idx + 1).min(colors.len() - 1);
    let t = scaled.fract();
    let t = t * t * (3.0 - 2.0 * t);

    let r = colors[idx].0 as f32 + (colors[next].0 as f32 - colors[idx].0 as f32) * t;
    let g = colors[idx].1 as f32 + (colors[next].1 as f32 - colors[idx].1 as f32) * t;
    let b = colors[idx].2 as f32 + (colors[next].2 as f32 - colors[idx].2 as f32) * t;
    (r as u8, g as u8, b as u8)
}

fn apply_vertical_depth(color: (u8, u8, u8), row: usize, height: usize) -> (u8, u8, u8) {
    let factor = 1.0 - (row as f32 / height as f32 * 0.87);
    (
        (color.0 as f32 * factor) as u8,
        (color.1 as f32 * factor) as u8,
        (color.2 as f32 * factor) as u8,
    )
}
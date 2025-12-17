use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::tui::app::RebuildState;

pub(super) fn visible_output_lines(
    mut lines: Vec<Line<'static>>,
    text_area: Rect,
    rebuild: &mut RebuildState,
    total_lines: usize,
) -> (usize, Vec<Line<'static>>) {
    let content_height = text_area.height.max(1);
    let content_width = text_area.width;
    rebuild.viewport_height = content_height;
    rebuild.viewport_width = content_width;

    let viewport = usize::from(content_height);
    let max_start = total_lines.saturating_sub(viewport);
    let mut scroll_top = rebuild.scroll_y as usize;
    scroll_top = if rebuild.auto_scroll {
        max_start
    } else {
        scroll_top.min(max_start)
    };
    rebuild.scroll_y = u16::try_from(scroll_top).unwrap_or(u16::MAX);

    if lines.len() < viewport {
        let pad = if content_width == 0 {
            String::new()
        } else {
            " ".repeat(content_width as usize)
        };
        lines.resize_with(viewport, || Line::from(pad.clone()));
    }

    let start_index = scroll_top.min(max_start);
    let visible = lines.into_iter().skip(start_index).take(viewport).collect();
    (start_index, visible)
}

pub(super) fn render_gutter(
    frame: &mut Frame,
    number_area: Option<Rect>,
    start_index: usize,
    visible_len: usize,
    line_digits: usize,
    gutter_width: u16,
    total_lines: usize,
) {
    let Some(number_area) = number_area else {
        return;
    };

    let separator = if gutter_width > u16::try_from(line_digits).unwrap_or(0) {
        " "
    } else {
        ""
    };
    let blank_label = format!(
        "{:>width$}{separator}",
        "",
        width = line_digits,
        separator = separator
    );
    let mut number_lines: Vec<Line> = Vec::with_capacity(visible_len);
    for offset in 0..visible_len {
        let idx = start_index + offset;
        let label = if idx < total_lines {
            format!(
                "{:>width$}{separator}",
                idx + 1,
                width = line_digits,
                separator = separator
            )
        } else {
            blank_label.clone()
        };
        number_lines.push(Line::from(vec![Span::styled(
            label,
            Style::default().fg(Color::DarkGray),
        )]));
    }
    frame.render_widget(Paragraph::new(number_lines), number_area);
}

pub(super) fn render_scrollbar(
    frame: &mut Frame,
    scrollbar_area: Rect,
    max_line_width: usize,
    content_width: u16,
    rebuild: &RebuildState,
) {
    if scrollbar_area.width == 0 {
        return;
    }

    let track_len = usize::from(scrollbar_area.width);
    let viewport_cols = usize::from(content_width).max(1);
    let max_offset = max_line_width.saturating_sub(viewport_cols);
    let total_width = max_line_width.max(viewport_cols);
    let mut thumb_len = (viewport_cols * track_len) / total_width;
    thumb_len = thumb_len.clamp(1, track_len);
    let track_range = track_len.saturating_sub(thumb_len);
    let scroll_offset = usize::from(rebuild.scroll_x).min(max_offset);
    let thumb_start = if max_offset == 0 || track_range == 0 {
        0
    } else {
        (scroll_offset * track_range + max_offset / 2) / max_offset
    };
    let mut spans = Vec::with_capacity(track_len);
    for idx in 0..track_len {
        if idx >= thumb_start && idx < thumb_start + thumb_len {
            spans.push(Span::styled("⠶", Style::default().fg(Color::Cyan)));
        } else {
            spans.push(Span::styled("─", Style::default().fg(Color::DarkGray)));
        }
    }
    let bar = Paragraph::new(Line::from(spans));
    frame.render_widget(bar, scrollbar_area);
}

pub(super) fn clear_prompt(frame: &mut Frame, prompt_area: Option<Rect>) {
    if let Some(prompt_area) = prompt_area {
        frame.render_widget(Clear, prompt_area);
    }
}

use crate::tui::app::state::{App, Msg};
use unicode_width::UnicodeWidthChar;

pub(super) fn handle_scroll_message(app: &mut App, msg: &Msg) {
    match msg {
        Msg::ScrollOutputUp => adjust_vertical_scroll(app, -1),
        Msg::ScrollOutputDown => adjust_vertical_scroll(app, 1),
        Msg::ScrollOutputPageUp => adjust_vertical_scroll(app, -12),
        Msg::ScrollOutputPageDown => adjust_vertical_scroll(app, 12),
        Msg::ScrollOutputTop => set_vertical_scroll(app, 0),
        Msg::ScrollOutputBottom => set_vertical_to_bottom(app),
        Msg::ScrollOutputLeft => adjust_horizontal_scroll(app, -4),
        Msg::ScrollOutputRight => adjust_horizontal_scroll(app, 4),
        _ => {}
    }
}

pub(super) fn adjust_vertical_scroll(app: &mut App, delta: i32) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        rebuild.auto_scroll = false;
        let current = i32::from(rebuild.scroll_y);
        let mut next = current + delta;
        if next < 0 {
            next = 0;
        }
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let max_scroll = clamp_usize_to_i32(job.output.len().saturating_sub(viewport));
        if max_scroll >= 0 {
            next = next.min(max_scroll);
        }
        rebuild.scroll_y = clamp_i32_to_u16(next);
    }
}

pub(super) fn set_vertical_scroll(app: &mut App, value: u16) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let max_scroll = clamp_usize_to_u16(
            rebuild
                .jobs
                .get(rebuild.active_idx)
                .map_or(0, |job| job.output.len().saturating_sub(viewport)),
        );
        rebuild.scroll_y = value.min(max_scroll);
        rebuild.auto_scroll = false;
    }
}

pub(super) fn set_vertical_to_bottom(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let bottom = clamp_usize_to_u16(job.output.len().saturating_sub(viewport));
        rebuild.scroll_y = bottom;
        rebuild.auto_scroll = true;
    }
}

pub(super) fn adjust_horizontal_scroll(app: &mut App, delta: i32) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        let viewport = usize::from(rebuild.viewport_width.max(1));
        let max_line_width = job
            .output
            .iter()
            .map(|entry| line_display_width(&entry.text))
            .max()
            .unwrap_or(0);
        let (max_offset, step) = if max_line_width == 0 {
            (0usize, 0usize)
        } else if max_line_width > viewport {
            (
                max_line_width.saturating_sub(viewport),
                (viewport * 2 / 3).max(1),
            )
        } else {
            let target = max_line_width.saturating_sub(1).min(4);
            (target, target.max(1))
        };

        if max_offset == 0 {
            rebuild.scroll_x = 0;
            rebuild.auto_scroll = false;
            return;
        }

        let current = usize::from(rebuild.scroll_x);
        let mut next = if delta >= 0 {
            current.saturating_add(step)
        } else {
            current.saturating_sub(step)
        };
        if delta >= 0 {
            next = next.min(max_offset);
        }
        rebuild.scroll_x = clamp_usize_to_u16(next);
        rebuild.auto_scroll = false;
    }
}

pub(super) fn clamp_usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

pub(super) fn clamp_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

pub(super) fn clamp_i32_to_u16(value: i32) -> u16 {
    let non_negative = value.max(0);
    u16::try_from(non_negative).unwrap_or(u16::MAX)
}

pub(super) fn line_display_width(text: &str) -> usize {
    let segment = text.rsplit('\r').next().unwrap_or(text);
    segment
        .replace('\t', "    ")
        .chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
        .sum()
}

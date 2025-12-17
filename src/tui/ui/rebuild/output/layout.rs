use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::tui::app::SearchState;

use super::super::output_lines::count_digits;
use super::super::search_prompt::search_prompt_visible;

pub(super) struct OutputLayout {
    pub text_area: Rect,
    pub number_area: Option<Rect>,
    pub prompt_area: Option<Rect>,
    pub gutter_width: u16,
    pub line_digits: usize,
}

pub(super) struct ScrollbarAreas {
    pub text: Rect,
    pub numbers: Option<Rect>,
    pub bar: Option<Rect>,
}

pub(super) fn base_output_layout(
    area: Rect,
    total_lines: usize,
    search_state: Option<&SearchState>,
) -> Option<OutputLayout> {
    let line_digits = count_digits(total_lines.max(1)).max(3);
    let line_digits_u16 = u16::try_from(line_digits).unwrap_or(u16::MAX);
    let mut gutter_width = u16::try_from(line_digits + 1).unwrap_or(u16::MAX);
    if gutter_width >= area.width {
        gutter_width = line_digits_u16;
        if gutter_width >= area.width {
            gutter_width = 0;
        }
    }

    let mut text_area = area;
    let mut number_area = None;
    if gutter_width > 0 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(gutter_width), Constraint::Min(1)])
            .split(area);
        number_area = Some(split[0]);
        text_area = split[1];
    }

    if text_area.width == 0 || text_area.height == 0 {
        return None;
    }

    let mut prompt_area = None;
    if search_state.is_some_and(search_prompt_visible) && text_area.height > 1 {
        let segments = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(text_area);
        text_area = segments[0];
        prompt_area = Some(segments[1]);
        number_area = number_area.map(|rect| {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(rect)[0]
        });
    }

    if text_area.width == 0 || text_area.height == 0 {
        return None;
    }

    Some(OutputLayout {
        text_area,
        number_area,
        prompt_area,
        gutter_width,
        line_digits,
    })
}

pub(super) fn adjust_for_scrollbar(
    text_area: Rect,
    number_area: Option<Rect>,
    max_line_width: usize,
) -> ScrollbarAreas {
    if max_line_width <= usize::from(text_area.width) || text_area.height <= 1 {
        return ScrollbarAreas {
            text: text_area,
            numbers: number_area,
            bar: None,
        };
    }

    let segments = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(text_area);
    let text_area = segments[0];
    let scrollbar_area = segments[1];
    let number_area = number_area.map(|rect| {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(rect)[0]
    });

    ScrollbarAreas {
        text: text_area,
        numbers: number_area,
        bar: Some(scrollbar_area),
    }
}

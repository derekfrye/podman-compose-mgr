use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::{RebuildJob, RebuildState};

use super::super::{output_lines::build_output_lines, search_prompt::draw_search_prompt};
use super::{
    layout::{adjust_for_scrollbar, base_output_layout},
    scroll::{clear_prompt, render_gutter, render_scrollbar, visible_output_lines},
};

pub(crate) fn draw_rebuild_output(frame: &mut Frame, area: Rect, rebuild: &mut RebuildState) {
    let Some((header, output_len)) = rebuild
        .jobs
        .get(rebuild.active_idx)
        .map(|job| (job_header(job), job.output.len()))
    else {
        let empty = Paragraph::new("Waiting for jobs...")
            .block(Block::default().title("Output").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    frame.render_widget(Clear, area);
    let block = Block::default().title(header).borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.width == 0 || inner_area.height == 0 {
        return;
    }

    let search_state = rebuild.search.clone();
    let search_ref = search_state.as_ref();
    let Some(mut layout) = base_output_layout(inner_area, output_len, search_ref) else {
        return;
    };

    let (lines, max_line_width) = {
        let job = &rebuild.jobs[rebuild.active_idx];
        build_output_lines(job, search_ref, layout.text_area.width)
    };

    let scrollbar = adjust_for_scrollbar(layout.text_area, layout.number_area, max_line_width);
    layout.text_area = scrollbar.text;
    layout.number_area = scrollbar.numbers;

    if layout.text_area.width == 0 || layout.text_area.height == 0 {
        clear_prompt(frame, layout.prompt_area);
        return;
    }

    let (start_index, visible) = visible_output_lines(lines, layout.text_area, rebuild, output_len);

    render_gutter(
        frame,
        layout.number_area,
        start_index,
        visible.len(),
        layout.line_digits,
        layout.gutter_width,
        output_len,
    );

    let paragraph = Paragraph::new(visible).scroll((0, rebuild.scroll_x));
    frame.render_widget(paragraph, layout.text_area);

    if let Some(prompt_area) = layout.prompt_area {
        if let Some(search) = search_ref {
            draw_search_prompt(frame, prompt_area, search);
        } else {
            frame.render_widget(Clear, prompt_area);
        }
    }

    if let Some(scrollbar_area) = scrollbar.bar {
        render_scrollbar(
            frame,
            scrollbar_area,
            max_line_width,
            layout.text_area.width,
            rebuild,
        );
    }
}

fn job_header(job: &RebuildJob) -> String {
    match &job.container {
        Some(container) => format!("{} ({container})", job.image),
        None => job.image.clone(),
    }
}

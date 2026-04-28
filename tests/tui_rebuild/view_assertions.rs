use crate::context::TestContext;
use podman_compose_mgr::tui::app::{self, Msg, OutputStream};
use ratatui::buffer::Buffer;
use ratatui::widgets::Widget;
use std::rc::Rc;

pub(crate) fn verify_manual_rendering(
    ctx: &mut TestContext,
    list_buffer: &Buffer,
    active_header: &str,
) {
    let rebuild_chunks = rebuild_layout(ctx.width, ctx.height);
    let rebuild_state = ctx
        .app
        .rebuild
        .as_ref()
        .expect("rebuild state for manual render");
    let job = &rebuild_state.jobs[rebuild_state.active_idx];
    let header = active_header.to_string();
    let trimmed_lines: Vec<_> = job
        .output
        .iter()
        .take(2)
        .cloned()
        .map(|entry| match entry.stream {
            OutputStream::Stdout => {
                ratatui::text::Line::from(vec![ratatui::text::Span::raw(entry.text)])
            }
            OutputStream::Stderr => ratatui::text::Line::from(vec![ratatui::text::Span::styled(
                entry.text,
                ratatui::style::Style::default().fg(ratatui::style::Color::LightRed),
            )]),
        })
        .collect();

    let make_paragraph = || {
        ratatui::widgets::Paragraph::new(trimmed_lines.clone())
            .block(
                ratatui::widgets::Block::default()
                    .title(header.clone())
                    .borders(ratatui::widgets::Borders::ALL),
            )
            .scroll((rebuild_state.scroll_y, rebuild_state.scroll_x))
    };

    let mut stale_buffer = list_buffer.clone();
    make_paragraph().render(rebuild_chunks[0], &mut stale_buffer);
    let stale_view = ctx.buffer_to_string(&stale_buffer);
    assert!(
        !stale_view.contains("Podman container for rclone"),
        "manual render should overwrite stale list content"
    );

    let mut cleared_buffer = list_buffer.clone();
    ratatui::widgets::Clear.render(rebuild_chunks[0], &mut cleared_buffer);
    make_paragraph().render(rebuild_chunks[0], &mut cleared_buffer);
    let cleared_view = ctx.buffer_to_string(&cleared_buffer);
    assert!(
        !cleared_view.contains("Podman container for rclone"),
        "clearing the pane before rendering should remove stale table text"
    );
}

pub(crate) fn verify_rebuild_view(ctx: &mut TestContext, active_header: &str) {
    let (base_buffer, base_view) = ctx.capture_view();
    println!("\n===== Rebuild View (Top) =====\n{base_view}");
    assert!(
        base_view.contains(active_header),
        "output pane should render active job header"
    );
    assert!(base_view.contains("Legend"), "legend pane should render");
    assert!(
        base_view.contains("Job: 1/2"),
        "sidebar should show job position"
    );
    assert!(
        base_view.contains("Status: Done"),
        "sidebar should show completed status"
    );
    assert!(
        base_view.contains("Refresh djf/ddns"),
        "output pane should include prompt text"
    );
    assert!(
        base_view.contains("STEP 1/10: FROM alpine:latest"),
        "output pane should render streamed podman output"
    );

    let _ = base_buffer;
}

pub(crate) fn verify_scroll_down(ctx: &mut TestContext) {
    for _ in 0..90 {
        app::update_with_services(&mut ctx.app, Msg::ScrollOutputDown, Some(&ctx.services));
    }

    let (_, bottom_view) = ctx.capture_view();
    println!("===== Rebuild View (90 Down Arrows) =====\n{bottom_view}");
}

pub(crate) fn verify_work_queue_modal(ctx: &mut TestContext, job_images: &[String]) {
    app::update_with_services(&mut ctx.app, Msg::OpenWorkQueue, Some(&ctx.services));
    let (_, modal_view) = ctx.capture_view();
    println!("===== Work Queue Modal =====\n{modal_view}");
    assert!(
        modal_view.contains("Work Queue (Esc=close)"),
        "work queue modal title should render"
    );
    assert!(
        modal_view.contains(&format!("▶ {}", job_images[0])),
        "work queue should highlight ddns job"
    );
    assert!(
        modal_view.contains(&job_images[1]),
        "work queue should list rclone job"
    );
}

fn rebuild_layout(width: u16, height: u16) -> Rc<[ratatui::layout::Rect]> {
    let root_area = ratatui::layout::Rect::new(0, 0, width, height);
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
            ratatui::layout::Constraint::Length(3),
            ratatui::layout::Constraint::Min(3),
        ])
        .split(root_area);

    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Min(40),
            ratatui::layout::Constraint::Length(24),
        ])
        .split(chunks[1])
}

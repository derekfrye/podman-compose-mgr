use crate::context::{TestContext, wait_for_rebuild};
use crate::view_assertions::{
    verify_manual_rendering, verify_rebuild_view, verify_scroll_down, verify_work_queue_modal,
};
use podman_compose_mgr::tui::app::{self, App, Msg, RebuildJob, Services, UiState};

#[test]
fn tui_rebuild_all_streams_output_and_scrolls_to_top() {
    let mut ctx = TestContext::new();
    ctx.seed_scan_results();

    let (list_buffer, list_view) = ctx.capture_view();
    assert_initial_list_view(&list_view);

    wait_for_rebuild(&mut ctx.app, &ctx.services, &ctx.rx);
    assert_app_in_rebuild(&ctx.app);

    let rebuild = ctx.app.rebuild.as_ref().expect("rebuild state expected");
    assert_eq!(rebuild.jobs.len(), 2, "expected two rebuild jobs");
    assert!(
        rebuild
            .jobs
            .iter()
            .all(|job| job.status == podman_compose_mgr::tui::app::RebuildStatus::Succeeded),
        "all jobs should succeed"
    );

    assert_ddns_job(&rebuild.jobs[0]);
    assert_rclone_job(&rebuild.jobs[1]);

    let (active_header, job_images) = reset_scroll_and_verify(&mut ctx.app, &ctx.services);
    verify_manual_rendering(&mut ctx, &list_buffer, &active_header);
    verify_rebuild_view(&mut ctx, &active_header);
    verify_scroll_down(&mut ctx);
    verify_work_queue_modal(&mut ctx, &job_images);
}

fn assert_initial_list_view(list_view: &str) {
    assert!(
        list_view.contains("Podman container for rclone"),
        "fixtures should include a second table row for rclone"
    );
}

fn assert_app_in_rebuild(app: &App) {
    assert_eq!(
        app.state,
        UiState::Rebuilding,
        "app should remain in rebuild view"
    );
}

fn assert_ddns_job(job: &RebuildJob) {
    assert!(
        !job.output.is_empty(),
        "ddns job should record at least the prompt output"
    );
    let prompt = job
        .output
        .front()
        .expect("ddns job output should include prompt");
    assert!(
        prompt.text.starts_with("Refresh djf/ddns"),
        "ddns prompt should be visible at start of output"
    );
    assert!(
        prompt.text.contains("p/N/d/b/s/?:"),
        "ddns prompt should include action shortcuts"
    );
    assert_job_output(job, "Auto-selecting 'b' (build)", "auto build selection");
    assert_job_output(job, "STEP 1/10: FROM alpine:latest", "podman STEP output");
    assert_job_output(
        job,
        "apk add jq curl bind-tools tini",
        "build command output",
    );
}

fn assert_rclone_job(job: &RebuildJob) {
    assert!(
        job.output
            .iter()
            .any(|line| line.text.starts_with("Refresh djf/rclone")),
        "rclone job should display rebuild prompt"
    );
    assert_job_output(job, "Auto-selecting 'b' (build)", "auto build selection");
    assert_job_output(job, "Rebuild queue completed", "queue completion");
    assert_job_output(job, "STEP 1/8: FROM fedora:42", "podman STEP output");
    assert_job_output(
        job,
        "Updating and loading repositories",
        "repository update",
    );
}

fn assert_job_output(job: &RebuildJob, needle: &str, label: &str) {
    assert!(
        job.output.iter().any(|line| line.text.contains(needle)),
        "{label} missing from job output"
    );
}

fn reset_scroll_and_verify(app: &mut App, services: &Services) -> (String, Vec<String>) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        rebuild.active_idx = 0;
    }
    app::update_with_services(app, Msg::ScrollOutputBottom, Some(services));
    app::update_with_services(app, Msg::ScrollOutputTop, Some(services));

    let rebuild_state = app.rebuild.as_ref().expect("rebuild state after scroll");
    assert_eq!(
        rebuild_state.scroll_y, 0,
        "scroll should point to first line"
    );
    let first_line = rebuild_state.jobs[0]
        .output
        .front()
        .expect("ddns job should have output");
    assert!(
        first_line.text.starts_with("Refresh djf/ddns"),
        "scroll to top should reveal prompt line"
    );

    let active_header = match (
        &rebuild_state.jobs[0].image,
        &rebuild_state.jobs[0].container,
    ) {
        (image, Some(container)) => format!("{image} ({container})"),
        (image, None) => image.clone(),
    };
    let job_images = rebuild_state
        .jobs
        .iter()
        .map(|job| job.image.clone())
        .collect();

    (active_header, job_images)
}

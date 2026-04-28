use std::path::Path;

use podman_compose_mgr::args::types::{Args, OneShotArgs, SimulateViewMode, TuiArgs};
use podman_compose_mgr::tui::app::App;
use podman_compose_mgr::tui::ui;

pub(crate) const GOLDEN_JSON: &str = "tests/test08/golden.json";

pub(crate) fn tui_args(root: &Path, include: Vec<String>) -> Args {
    Args {
        config_toml: None,
        path: root.to_path_buf(),
        verbose: 0,
        exclude_path_patterns: vec![],
        include_path_patterns: include,
        build_args: vec![],
        temp_file_path: std::env::temp_dir(),
        podman_bin: None,
        no_cache: false,
        one_shot: OneShotArgs::default(),
        tui: TuiArgs {
            enabled: true,
            ..TuiArgs::default()
        },
        rebuild_view_line_buffer_max:
            podman_compose_mgr::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT,
        tui_simulate: None,
        tui_simulate_podman_input_json: None,
    }
}

pub(crate) fn simulated_dockerfile_args(root: &Path, include: Vec<String>) -> Args {
    Args {
        one_shot: OneShotArgs {
            one_shot: true,
            dry_run: true,
        },
        tui: TuiArgs::default(),
        tui_simulate: Some(SimulateViewMode::Dockerfile),
        tui_simulate_podman_input_json: Some(GOLDEN_JSON.into()),
        ..tui_args(root, include)
    }
}

pub(crate) fn render_app(app: &mut App, args: &Args, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui::draw(f, app, args)).expect("draw");
    let buffer = terminal.backend_mut().buffer().clone();
    let mut rendered = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).expect("cell exists");
            rendered.push_str(cell.symbol());
        }
        rendered.push('\n');
    }
    println!("{rendered}");
    rendered
}

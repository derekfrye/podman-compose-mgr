use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::render_support::{GOLDEN_JSON, simulated_dockerfile_args};
use podman_compose_mgr::infra::discovery_adapter::FsDiscovery;
use podman_compose_mgr::ports::DiscoveryPort;
use podman_compose_mgr::tui::simulate_view_with_ports;
use podman_compose_mgr::utils::log_utils::Logger;
use tempfile::tempdir;

#[test]
fn simulate_dockerfile_view_emits_registry_matches() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path();
    let help_dir = root.join("help");
    fs::create_dir_all(&help_dir).expect("create help dir");
    fs::write(help_dir.join("Dockerfile.ffmpeg"), "FROM scratch\n").expect("write ffmpeg");
    fs::write(help_dir.join("Dockerfile.ffmpeg_base"), "FROM scratch\n")
        .expect("write ffmpeg base");

    let include = format!("^{}", regex::escape(help_dir.to_str().expect("utf8 path")));
    let args = simulated_dockerfile_args(root, vec![format!("{include}.*")]);
    let discovery: Arc<dyn DiscoveryPort> = Arc::new(FsDiscovery);
    let logger = Logger::new(0);

    let mut buf = Vec::new();
    simulate_view_with_ports(
        &args,
        podman_compose_mgr::args::types::SimulateViewMode::Dockerfile,
        &logger,
        discovery,
        podman_compose_mgr::tui::podman_from_json(PathBuf::from(GOLDEN_JSON).as_path())
            .expect("json podman"),
        &mut buf,
    )
    .expect("simulate dockerfile view");

    let out = String::from_utf8(buf).expect("utf8 output");
    assert!(
        out.contains("Dockerfile.ffmpeg -> registry matched (more than one Dockerfile in the dir) / registry name matched localhost/djf/ffmpeg:latest"),
        "ffmpeg line should include registry match\n{out}"
    );
    assert!(
        out.contains("Dockerfile.ffmpeg_base -> registry matched (more than one Dockerfile in the dir) / registry name matched localhost/djf/ffmpeg_base:latest"),
        "ffmpeg_base line should include registry match\n{out}"
    );
}

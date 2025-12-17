use super::*;
use crate::image_build::rebuild::recording_logger::RecordingLogger;
use crate::interfaces::MockCommandHelper;
use tempfile::tempdir;
use walkdir::WalkDir;

#[test]
fn start_selected_build_falls_back_to_pull_when_no_buildfiles_found() {
    let temp = tempdir().expect("tempdir");
    let compose_path = temp.path().join("docker-compose.yml");
    std::fs::write(&compose_path, "version: '3'\nservices: {}\n").expect("write compose");

    let entry = WalkDir::new(&compose_path)
        .max_depth(0)
        .into_iter()
        .next()
        .expect("walkdir entry")
        .expect("dir entry");

    let selection = RebuildSelection::new("example:latest", "example");
    let grammars: Vec<GrammarFragment> = Vec::new();
    let build_args: Vec<String> = Vec::new();
    let logger = RecordingLogger::default();

    let mut cmd = MockCommandHelper::new();
    cmd.expect_exec_cmd()
        .withf(|cmd, args| {
            cmd == "podman" && args.len() == 2 && args[0] == "pull" && args[1] == "example:latest"
        })
        .times(1)
        .returning(|_, _| Ok(()));

    let context = UserChoiceContext {
        entry: &entry,
        selection,
        build_args: &build_args,
        grammars: &grammars,
        logger: &logger,
        no_cache: false,
    };

    start_selected_build(&cmd, &context).expect("fallback pull succeeds");

    let logs = logger.logs();
    assert!(
        logs.iter().any(|(level, message)| {
            *level == BuildLogLevel::Info && message.contains("falling back to `podman pull`")
        }),
        "expected fallback info log, got {logs:?}"
    );
}

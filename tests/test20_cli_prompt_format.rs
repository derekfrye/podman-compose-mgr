use podman_compose_mgr::args::Args;
use podman_compose_mgr::image_build::container_file::parse_container_file;
use podman_compose_mgr::image_build::rebuild::{build_rebuild_grammars, read_yaml_file};
use podman_compose_mgr::interfaces::DefaultCommandHelper;
use podman_compose_mgr::read_interactive_input::{
    StdinHelperWrapper, TestStdinHelper, do_prompt_formatting,
    read_val_from_cmd_line_and_proceed_with_deps, unroll_grammar_into_string,
};
use walkdir::WalkDir;

#[test]
fn prompt_formatting_widths_match_for_all_items() {
    let args = Args {
        path: std::path::PathBuf::from("tests/test1"),
        ..Default::default()
    };

    // Discover prompt items similar to CLI MVU discovery
    let mut items: Vec<(std::path::PathBuf, String, String)> = Vec::new();
    for entry in WalkDir::new(&args.path).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(pstr) = entry.path().to_str() else {
            continue;
        };
        if entry.file_name() == "docker-compose.yml" {
            if let Ok(yaml) = read_yaml_file(pstr)
                && let Some(services) = yaml.get("services").and_then(|v| v.as_mapping())
            {
                for (_, svc_cfg) in services {
                    let Some(m) = svc_cfg.as_mapping() else {
                        continue;
                    };
                    let Some(img) = m.get("image").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let Some(container) = m.get("container_name").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    items.push((
                        entry.path().to_path_buf(),
                        img.to_string(),
                        container.to_string(),
                    ));
                }
            }
        } else if entry.path().extension().and_then(|s| s.to_str()) == Some("container")
            && let Ok(info) = parse_container_file(entry.path())
        {
            let container = info.name.unwrap_or_else(|| {
                entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });
            items.push((entry.path().to_path_buf(), info.image, container));
        }
    }

    // For each item, format prompt at widths 60 and 40 and ensure it contains key tokens
    for (path, image, container) in items {
        let entry = WalkDir::new(&path)
            .into_iter()
            .filter_map(Result::ok)
            .find(|e| e.path() == path)
            .expect("entry must exist");

        let mut g = build_rebuild_grammars(&entry, &image, &container);
        do_prompt_formatting(&mut g, 60);
        let s60 = unroll_grammar_into_string(&g, false, true);
        assert!(s60.contains("p/N/d/b/s/?:"));
        assert!(s60.contains("from"));
        assert!(
            s60.len() <= 60,
            "prompt at width 60 should be <= 60 chars, got {}: {}",
            s60.len(),
            s60
        );

        let mut g2 = build_rebuild_grammars(&entry, &image, &container);
        do_prompt_formatting(&mut g2, 40);
        let s40 = unroll_grammar_into_string(&g2, false, true);
        assert!(s40.contains("p/N/d/b/s/?:"));
        assert!(s40.contains("from"));

        // Ensure total lengths do not exceed target widths
        assert!(
            s40.len() <= 40,
            "prompt at width 40 should be <= 40 chars, got {}: {}",
            s40.len(),
            s40
        );

        // Check that the image name was shortened as expected for 40 cols.
        // Based on current formatter, images are shortened to first char + "..." when space is tight.
        // Hard-code expected shortened prefixes for our test fixtures.
        if image.starts_with("djf/") {
            assert!(
                s40.contains("d..."),
                "expected shortened image 'd...' in 40-col prompt: {}",
                s40
            );
        } else if image.starts_with("pihole/") {
            assert!(
                s40.contains("p..."),
                "expected shortened image 'p...' in 40-col prompt: {}",
                s40
            );
        }
    }

    // Also verify exact strings for a stable, known fixture to catch spacing regressions.
    // Fixture: tests/test1/image1/docker-compose.yml, service "rust" -> image "djf/rusty-golf", container "golf".
    let fixture_path = std::path::PathBuf::from("tests/test1/image1/docker-compose.yml");
    let entry = WalkDir::new(&fixture_path)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| e.path() == fixture_path)
        .expect("fixture entry must exist");

    let image = "djf/rusty-golf".to_string();
    let container = "golf".to_string();

    let mut g60 = build_rebuild_grammars(&entry, &image, &container);
    do_prompt_formatting(&mut g60, 60);
    let s60 = unroll_grammar_into_string(&g60, false, true);

    let mut g40 = build_rebuild_grammars(&entry, &image, &container);
    do_prompt_formatting(&mut g40, 40);
    let s40 = unroll_grammar_into_string(&g40, false, true);

    // For cross-platform consistency, account for path separator in 60-col expected.
    let sep = std::path::MAIN_SEPARATOR;
    let sep = if sep == '\\' { "\\" } else { "/" };
    let expected_s60 = format!("Refresh djf/rusty-g... from ...test1{sep}image1? p/N/d/b/s/?: ");
    let expected_s40 = "Refresh d... from ...e1? p/N/d/b/s/?: ";
    assert_eq!(s60, expected_s60, "Exact 60-col prompt mismatch");
    assert_eq!(s40, expected_s40, "Exact 40-col prompt mismatch");
}

#[test]
fn pressing_enter_selects_default_choice() {
    let fixture_path = std::path::PathBuf::from("tests/test1/image1/docker-compose.yml");
    let entry = WalkDir::new(&fixture_path)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| e.path() == fixture_path)
        .expect("fixture entry must exist");

    let image = "djf/rusty-golf".to_string();
    let container = "golf".to_string();

    let mut grammars = build_rebuild_grammars(&entry, &image, &container);
    let stdin_helper = StdinHelperWrapper::Test(TestStdinHelper {
        response: String::new(),
    });
    let result = read_val_from_cmd_line_and_proceed_with_deps(
        &mut grammars,
        &DefaultCommandHelper,
        Box::new(|_| {}),
        Some(80),
        Some(&stdin_helper),
    );

    assert_eq!(result.user_entered_val.as_deref(), Some("N"));
    assert!(!result.was_interrupted);
}

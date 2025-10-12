use std::path::{Path, PathBuf};

use podman_compose_mgr::image_build::container_file::parse_container_file;
use podman_compose_mgr::image_build::rebuild::{build_rebuild_grammars, read_yaml_file};
use podman_compose_mgr::interfaces::DefaultCommandHelper;
use podman_compose_mgr::read_interactive_input::{
    StdinHelperWrapper, TestStdinHelper, do_prompt_formatting,
    read_val_from_cmd_line_and_proceed_with_deps, unroll_grammar_into_string,
};
use walkdir::{DirEntry, WalkDir};

fn collect_prompt_items(base: &Path) -> Vec<(PathBuf, String, String)> {
    let mut items = Vec::new();
    for entry in WalkDir::new(base).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() == "docker-compose.yml" {
            collect_from_compose(&entry, &mut items);
        } else if entry.path().extension().and_then(|s| s.to_str()) == Some("container") {
            collect_from_container_file(&entry, &mut items);
        }
    }
    items
}

fn collect_from_compose(entry: &DirEntry, items: &mut Vec<(PathBuf, String, String)>) {
    let Some(path_str) = entry.path().to_str() else {
        return;
    };

    if let Ok(yaml) = read_yaml_file(path_str)
        && let Some(services) = yaml.get("services").and_then(|v| v.as_mapping())
    {
        for service_cfg in services.values() {
            let Some(mapping) = service_cfg.as_mapping() else {
                continue;
            };
            let Some(image) = mapping.get("image").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(container) = mapping.get("container_name").and_then(|v| v.as_str()) else {
                continue;
            };
            items.push((
                entry.path().to_path_buf(),
                image.to_string(),
                container.to_string(),
            ));
        }
    }
}

fn collect_from_container_file(entry: &DirEntry, items: &mut Vec<(PathBuf, String, String)>) {
    if let Ok(info) = parse_container_file(entry.path()) {
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

fn find_entry(path: &Path) -> DirEntry {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .find(|e| e.path() == path)
        .expect("entry must exist")
}

fn assert_prompt_layouts(entry: &DirEntry, image: &str, container: &str) {
    let mut wide = build_rebuild_grammars(entry, image, container);
    do_prompt_formatting(&mut wide, 60);
    let s60 = unroll_grammar_into_string(&wide, false, true);
    assert!(s60.contains("p/N/d/b/s/?:"));
    assert!(s60.contains("from"));
    assert!(
        s60.len() <= 60,
        "prompt at width 60 should be <= 60 chars, got {}: {}",
        s60.len(),
        s60
    );

    let mut narrow = build_rebuild_grammars(entry, image, container);
    do_prompt_formatting(&mut narrow, 40);
    let s40 = unroll_grammar_into_string(&narrow, false, true);
    assert!(s40.contains("p/N/d/b/s/?:"));
    assert!(s40.contains("from"));
    assert!(
        s40.len() <= 40,
        "prompt at width 40 should be <= 40 chars, got {}: {}",
        s40.len(),
        s40
    );

    if image.starts_with("djf/") {
        assert!(
            s40.contains("d..."),
            "expected shortened image 'd...' in 40-col prompt: {s40}"
        );
    } else if image.starts_with("pihole/") {
        assert!(
            s40.contains("p..."),
            "expected shortened image 'p...' in 40-col prompt: {s40}"
        );
    }
}

fn assert_fixture_layout(base: &Path) {
    let fixture_path = base.join("image1").join("docker-compose.yml");
    let entry = find_entry(&fixture_path);
    let image = "djf/rusty-golf";
    let container = "golf";

    let mut g60 = build_rebuild_grammars(&entry, image, container);
    do_prompt_formatting(&mut g60, 60);
    let s60 = unroll_grammar_into_string(&g60, false, true);

    let mut g40 = build_rebuild_grammars(&entry, image, container);
    do_prompt_formatting(&mut g40, 40);
    let s40 = unroll_grammar_into_string(&g40, false, true);

    let sep = std::path::MAIN_SEPARATOR;
    let sep = if sep == '\\' { "\\" } else { "/" };
    let expected_s60 = format!("Refresh djf/rusty-g... from ...test1{sep}image1? p/N/d/b/s/?: ");
    let expected_s40 = "Refresh d... from ...e1? p/N/d/b/s/?: ";
    assert_eq!(s60, expected_s60, "Exact 60-col prompt mismatch");
    assert_eq!(s40, expected_s40, "Exact 40-col prompt mismatch");
}

#[test]
fn prompt_formatting_widths_match_for_all_items() {
    let base = PathBuf::from("tests/test1");
    for (path, image, container) in collect_prompt_items(&base) {
        let entry = find_entry(&path);
        assert_prompt_layouts(&entry, &image, &container);
    }

    assert_fixture_layout(&base);
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
